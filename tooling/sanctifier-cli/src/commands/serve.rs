use anyhow::{Context, Result};
use clap::Args;
use sanctifier_core::rules::RuleRegistry;
use sanctifier_core::SanctifyConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};
use sanctifier_core::analysis_cache::AnalysisCache;
use sanctifier_core::Analyzer;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use warp::Filter;

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
    registry: Arc<RuleRegistry>,
    analyzer: Arc<Analyzer>,
    cache: Arc<Mutex<AnalysisCache<serde_json::Value>>>,
}

pub fn exec(args: ServeArgs) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async { serve_async(args).await })
}

async fn serve_async(args: ServeArgs) -> Result<()> {
    let registry = Arc::new(RuleRegistry::with_default_rules());
    let state = AppState { registry };
    let config = sanctifier_core::SanctifyConfig::default();
    let analyzer = Arc::new(Analyzer::new(config));
    let cache = Arc::new(Mutex::new(AnalysisCache::new(100)));

    let state = AppState { analyzer, cache };

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .context("Invalid bind address")?;

    println!("Sanctifier HTTP server starting on http://{}", addr);
    println!("   POST /analyze (body: raw Rust source) — returns NDJSON findings");
    println!("   GET  /health");

    let state_filter = warp::any().map(move || state.clone());

    let analyze_route = warp::post()
        .and(warp::path("analyze"))
        .and(warp::body::bytes())
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
) -> Result<impl warp::Reply, warp::Rejection> {
    let source = body
        .get("contract")
        .and_then(|v| v.as_str())
        .ok_or_else(warp::reject::reject)?;

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

    // Check cache or analyze
    let cache_key = format!("{:x}", md5::compute(source));
    let analyzer = &state.analyzer;
    let findings = {
        let mut cache = state.cache.lock().unwrap();
        cache.get_or_analyze(&cache_key, source, || {
            let mut results = serde_json::Map::new();

            let collisions = analyzer.scan_storage_collisions(source);
            results.insert("storage_collisions".into(), serde_json::to_value(collisions).unwrap_or_default());

            let size_warnings = analyzer.analyze_ledger_size(source);
            results.insert("ledger_size_warnings".into(), serde_json::to_value(size_warnings).unwrap_or_default());

            let unsafe_patterns = analyzer.analyze_unsafe_patterns(source);
            results.insert("unsafe_patterns".into(), serde_json::to_value(unsafe_patterns).unwrap_or_default());

            let auth_gaps = analyzer.scan_auth_gaps(source);
            results.insert("auth_gaps".into(), serde_json::to_value(auth_gaps).unwrap_or_default());

            let panic_issues = analyzer.scan_panics(source);
            results.insert("panic_issues".into(), serde_json::to_value(panic_issues).unwrap_or_default());

            serde_json::Value::Object(results)
        })
    };

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
