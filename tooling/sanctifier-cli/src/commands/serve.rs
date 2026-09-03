use anyhow::{Context, Result};
use clap::Args;
use futures_util::StreamExt;
use sanctifier_core::{analysis_cache::AnalysisCache, RuleViolation, Analyzer, SanctifyConfig};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use warp::{multipart::FormData, Buf, Filter, Rejection, Reply};

#[derive(Args)]
pub struct ServeArgs {
    /// Port to bind to
    #[arg(short, long, default_value = "9100")]
    port: u16,

    /// Address to bind to
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,
}

#[derive(Clone)]
struct AppState {
    #[allow(dead_code)]
    registry: Arc<RuleRegistry>,
    analyzer: Arc<Analyzer>,
    cache: Arc<Mutex<AnalysisCache<Vec<RuleViolation>>>>,
}

pub fn exec(args: ServeArgs) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async { serve_async(args).await })
}

async fn serve_async(args: ServeArgs) -> Result<()> {
    let registry = Arc::new(RuleRegistry::with_default_rules());
    let config = SanctifyConfig::default();
    let analyzer = Arc::new(Analyzer::new(config));
    let cache = Arc::new(Mutex::new(AnalysisCache::new(100)));

    let state = AppState {
        registry,
        analyzer,
        cache,
    };

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .context("Invalid bind address")?;

    println!("Sanctifier HTTP server starting on http://{}", addr);
    println!("   POST /analyze (body: raw Rust source) — returns NDJSON findings");
    println!("   GET  /health");

    let state_filter = warp::any().map(move || state.clone());

    let analyze_route = warp::post()
        .and(warp::path("analyze"))
        .and(warp::body::json())
        .and(state_filter.clone())
        .and_then(handle_analyze);

    let health_route = warp::get()
        .and(warp::path("health"))
        .map(|| warp::reply::json(&serde_json::json!({"status": "ok"})));

    let routes = analyze_route.or(health_route).recover(handle_rejection);

    warp::serve(routes).run(addr).await;

    Ok(())
}

async fn handle_analyze(
    body: serde_json::Value,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    // Extract contract source from multipart form
    let parts: Vec<_> = form.collect().await;
    
    let mut contract_source = None;
    for part in parts {
        let mut part = part.map_err(|_| warp::reject::reject())?;
        if part.name() == "contract" {
            let mut bytes = part
                .data()
                .await
                .ok_or_else(|| warp::reject::reject())?
                .map_err(|_| warp::reject::reject())?;
            let remaining = bytes.remaining();
            contract_source = Some(
                String::from_utf8(bytes.copy_to_bytes(remaining).to_vec())
                    .map_err(|_| warp::reject::reject())?
            );
            break;
        }
    }

    let source = contract_source.ok_or_else(|| warp::reject::reject())?;

    // Write to temp file
    let temp_dir = tempfile::tempdir().map_err(|_| warp::reject::reject())?;
    let contract_path = temp_dir.path().join("contract.rs");

    let mut file = fs::File::create(&contract_path)
        .await
        .map_err(|_| warp::reject::reject())?;
    file.write_all(source.as_bytes())
        .await
        .map_err(|_| warp::reject::reject())?;
    file.flush().await.map_err(|_| warp::reject::reject())?;

    // Check cache
    let cache_key = format!("{:x}", md5::compute(&source));
    if let Ok(mut cache) = state.cache.lock() {
        if cache.is_cached(&cache_key, &source) {
            let cached = cache.get_or_analyze(&cache_key, &source, Vec::new);
            return Ok(warp::reply::json(&cached));
        }
    }

    // Analyze
    let findings = state.analyzer.run_rules(&source);

    // Cache result
    if let Ok(mut cache) = state.cache.lock() {
        cache.get_or_analyze(&cache_key, &source, || findings.clone());
    }

    Ok(warp::reply::json(&findings))
}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if err.is_not_found() {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Not found"})),
            warp::http::StatusCode::NOT_FOUND,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Internal server error"})),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}
