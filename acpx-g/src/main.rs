use std::sync::{Arc, RwLock};

use axum::{routing::delete, routing::get, routing::post, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, EnvFilter};

use acpx_g::watcher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    // Parse CLI args
    let cli = parse_cli_args();

    // Database
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:acpx-g.db?mode=rwc".to_string());
    let pool = Arc::new(acpx_g::db::init(&database_url).await?);

    let templates = Arc::new(RwLock::new(Vec::new()));

    let state = Arc::new(acpx_g::api::AppState {
        pool: pool.clone(),
        templates: templates.clone(),
    });

    // Start workflow directory watcher if configured
    if let Some(ref workflow_dir) = cli.workflow_dir {
        tracing::info!(dir = %workflow_dir, "starting workflow directory watcher");
        tokio::spawn(watcher::watch_directory(
            pool.clone(),
            templates,
            workflow_dir.clone(),
        ));
    }

    // Router
    let app = Router::new()
        // Health check (for load balancers / orchestrators)
        .route("/health", get(acpx_g::api::health_check))
        // API Docs
        .route("/api/v1/docs", get(acpx_g::api::list_api_docs))
        // Template API
        .route("/api/v1/templates", get(acpx_g::api::list_templates))
        .route(
            "/api/v1/templates/{name}/run",
            post(acpx_g::api::run_template),
        )
        // Workflow API
        .route("/api/v1/workflows", post(acpx_g::api::submit_workflow))
        .route("/api/v1/workflows", get(acpx_g::api::list_workflows))
        .route("/api/v1/workflows/{run_id}", get(acpx_g::api::get_workflow))
        .route(
            "/api/v1/workflows/{run_id}",
            delete(acpx_g::api::delete_workflow_run),
        )
        .route(
            "/api/v1/workflows/{run_id}/nodes/{node_id}/logs",
            get(acpx_g::api::get_node_logs),
        )
        // Frontend: serve / as index.html, then all static assets
        .fallback_service(
            ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static"))
                .append_index_html_on_directories(true),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    tracing::info!("acpx-g http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Graceful shutdown: on Ctrl+C, stop accepting new connections and
    // mark any running workflows as failed in the DB.
    let shutdown_pool = pool.clone();
    let shutdown = async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutting down gracefully...");
        // Mark any still-running workflows as failed
        let running =
            sqlx::query_as::<_, (String,)>("SELECT id FROM workflow_runs WHERE status = 'running'")
                .fetch_all(&*shutdown_pool)
                .await
                .unwrap_or_default();
        for (id,) in running {
            tracing::warn!(run_id = %id, "marking running workflow as failed due to shutdown");
            let _ = sqlx::query(
                "UPDATE workflow_runs SET status = 'failed', error_message = 'server shutdown', finished_at = datetime('now') WHERE id = ?",
            )
            .bind(&id)
            .execute(&*shutdown_pool)
            .await;
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    Ok(())
}

struct CliArgs {
    workflow_dir: Option<String>,
}

fn parse_cli_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut workflow_dir = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--workflow-dir" => {
                if i + 1 < args.len() {
                    workflow_dir = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--help" | "-h" => {
                eprintln!("acpx-g — DAG workflow engine\n");
                eprintln!("USAGE:");
                eprintln!("    acpx-g [OPTIONS]\n");
                eprintln!("OPTIONS:");
                eprintln!("    --workflow-dir <DIR>  Watch directory for workflow YAML files");
                eprintln!("    --help, -h            Show this help message\n");
                eprintln!("ENVIRONMENT:");
                eprintln!("    DATABASE_URL  SQLite connection string (default: sqlite:acpx-g.db?mode=rwc)");
                eprintln!("    PORT          HTTP server port (default: 3000)");
                eprintln!("    RUST_LOG      Log level (default: info)");
                std::process::exit(0);
            }
            _ => {
                if args[i].starts_with('-') {
                    tracing::warn!(arg = %args[i], "unknown argument, try --help");
                }
            }
        }
        i += 1;
    }
    CliArgs { workflow_dir }
}
