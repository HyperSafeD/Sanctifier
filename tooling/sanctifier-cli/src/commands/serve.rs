use anyhow::{Context, Result};
use clap::Args;
use sanctifier_core::{analysis_cache::AnalysisCache, Analyzer, SanctifyConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use warp::{multipart::FormData, Filter, Rejection, Reply};

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
    analyzer: Arc<Analyzer>,
    cache: Arc<AnalysisCache>,
}

pub fn exec(args: ServeArgs) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async { serve_async(args).await })
}

async fn serve_async(args: ServeArgs) -> Result<()> {
    let config = SanctifyConfig::default();
    let analyzer = Arc::new(Analyzer::new(config));
    let cache = Arc::new(AnalysisCache::new());

    let state = AppState { analyzer, cache };

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .context("Invalid bind address")?;

    println!("🚀 Sanctifier HTTP server starting on http://{}", addr);
    println!("   POST /analyze - Analyze contract source");
    println!("   GET /health - Health check");
    println!();

    let state_filter = warp::any().map(move || state.clone());

    let analyze_route = warp::post()
        .and(warp::path("analyze"))
        .and(warp::multipart::form().max_length(5 * 1024 * 1024)) // 5MB limit
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
    form: FormData,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    // Extract contract source from multipart form
    let parts: Vec<_> = form.collect().await;
    
    let mut contract_source = None;
    for part in parts {
        let part = part.map_err(|_| warp::reject::reject())?;
        if part.name() == "contract" {
            let bytes = part
                .data()
                .await
                .ok_or_else(|| warp::reject::reject())?
                .map_err(|_| warp::reject::reject())?;
            contract_source = Some(
                String::from_utf8(bytes.to_vec())
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
    if let Some(cached) = state.cache.get(&cache_key) {
        return Ok(warp::reply::json(&cached));
    }

    // Analyze
    let findings = state
        .analyzer
        .analyze_file(&contract_path)
        .map_err(|_| warp::reject::reject())?;

    // Cache result
    state.cache.insert(cache_key, findings.clone());

    Ok(warp::reply::json(&findings))
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
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
