use std::sync::{Arc, RwLock};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::{
    ListRunsResponse, NodeRun, NodeRunResponse, SubmitWorkflowRequest, SubmitWorkflowResponse,
    WorkflowRun, WorkflowRunResponse,
};
use crate::runner;

/// A workflow template discovered in the watch directory.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowTemplate {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub node_count: usize,
    pub file_path: String,
}

pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub templates: Arc<RwLock<Vec<WorkflowTemplate>>>,
}

// ─── POST /api/v1/workflows ──────────────────────────────────────

pub async fn submit_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitWorkflowRequest>,
) -> impl IntoResponse {
    let run_id = Uuid::now_v7().to_string();

    // Parse to get metadata
    let wf = match crate::schema::parse_workflow(&req.yaml) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid workflow YAML: {e}")})),
            );
        }
    };

    // Insert run record
    let run = WorkflowRun {
        id: run_id.clone(),
        workflow_name: wf.name,
        workflow_version: wf.version,
        yaml_content: req.yaml.clone(),
        status: "pending".to_string(),
        node_count: wf.nodes.len() as i64,
        started_at: None,
        finished_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        error_message: None,
    };

    if let Err(e) = run.insert(&state.pool).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to create run: {e}")})),
        );
    }

    // Insert node_run records
    for node in &wf.nodes {
        let node_run = NodeRun {
            id: Uuid::now_v7().to_string(),
            run_id: run_id.clone(),
            node_id: runner::node_id(node).to_string(),
            node_type: runner::node_type_name(node).to_string(),
            status: "pending".to_string(),
            attempt: 0,
            started_at: None,
            finished_at: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            error_message: None,
        };
        if let Err(e) = node_run.insert(&state.pool).await {
            tracing::error!(error = %e, "failed to insert node_run");
        }
    }

    // Start async execution
    runner::run_workflow(state.pool.clone(), run_id.clone(), req.yaml).await;

    (
        StatusCode::CREATED,
        Json(serde_json::json!(SubmitWorkflowResponse {
            run_id,
            status: "pending".to_string(),
        })),
    )
}

// ─── GET /api/v1/workflows ───────────────────────────────────────

pub async fn list_workflows(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match WorkflowRun::list(&state.pool, 50).await {
        Ok(runs) => {
            let response_runs: Vec<WorkflowRunResponse> =
                runs.into_iter().map(WorkflowRunResponse::from).collect();
            (
                StatusCode::OK,
                Json(serde_json::json!(ListRunsResponse {
                    runs: response_runs,
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        ),
    }
}

// ─── GET /api/v1/workflows/:run_id ───────────────────────────────

pub async fn get_workflow(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    match WorkflowRun::find_by_id(&state.pool, &run_id).await {
        Ok(Some(run)) => {
            let nodes = NodeRun::find_by_run(&state.pool, &run_id)
                .await
                .unwrap_or_default();
            let mut response = WorkflowRunResponse::from(run);
            response.nodes = nodes.into_iter().map(NodeRunResponse::from).collect();
            (StatusCode::OK, Json(serde_json::json!(response)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "workflow run not found"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        ),
    }
}

// ─── GET /api/v1/workflows/:run_id/nodes/:node_id/logs ───────────

pub async fn get_node_logs(
    State(state): State<Arc<AppState>>,
    Path((run_id, node_run_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match NodeRun::find_by_id(&state.pool, &node_run_id).await {
        Ok(Some(node)) => {
            if node.run_id != run_id {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "node not found in this run"})),
                );
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "node_id": node.node_id,
                    "status": node.status,
                    "attempt": node.attempt,
                    "stdout": node.stdout,
                    "stderr": node.stderr,
                    "exit_code": node.exit_code,
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "node not found"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        ),
    }
}

// ─── GET /api/v1/templates ───────────────────────────────────────

pub async fn list_templates(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let templates = state.templates.read().unwrap().clone();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "templates": templates })),
    )
}

// ─── POST /api/v1/templates/:name/run ────────────────────────────

pub async fn run_template(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let templates = state.templates.read().unwrap().clone();
    let template = match templates.iter().find(|t| t.name == name) {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "template not found"})),
            );
        }
    };

    let yaml_content = match std::fs::read_to_string(&template.file_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to read template file: {e}")})),
            );
        }
    };

    let wf = match crate::schema::parse_workflow(&yaml_content) {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid workflow: {e}")})),
            );
        }
    };

    let run_id = Uuid::now_v7().to_string();

    let run = WorkflowRun {
        id: run_id.clone(),
        workflow_name: wf.name.clone(),
        workflow_version: wf.version.clone(),
        yaml_content: yaml_content.clone(),
        status: "pending".to_string(),
        node_count: wf.nodes.len() as i64,
        started_at: None,
        finished_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        error_message: None,
    };

    if let Err(e) = run.insert(&state.pool).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        );
    }

    for node in &wf.nodes {
        let node_run = NodeRun {
            id: Uuid::now_v7().to_string(),
            run_id: run_id.clone(),
            node_id: runner::node_id(node).to_string(),
            node_type: runner::node_type_name(node).to_string(),
            status: "pending".to_string(),
            attempt: 0,
            started_at: None,
            finished_at: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            error_message: None,
        };
        if let Err(e) = node_run.insert(&state.pool).await {
            tracing::error!(error = %e, "failed to insert node_run");
        }
    }

    runner::run_workflow(state.pool.clone(), run_id.clone(), yaml_content).await;

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "run_id": run_id,
            "status": "pending",
            "template": name,
        })),
    )
}
