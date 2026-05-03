mod executor;
mod loader;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use anyhow::Context;
use sqlx::SqlitePool;
use tokio::sync::Semaphore;

use crate::db::WorkflowRun;
use crate::schema::NodeDef;

pub use loader::load_workflow;

const MAX_CONCURRENT_NODES: usize = 16;

/// Run a workflow to completion asynchronously.
/// This spawns the actual execution so the caller (HTTP handler) can return immediately.
pub async fn run_workflow(pool: Arc<SqlitePool>, run_id: String, yaml_content: String) {
    tokio::spawn(async move {
        if let Err(e) = execute_dag(pool.clone(), &run_id, &yaml_content).await {
            tracing::error!(run_id = %run_id, error = %e, "workflow execution failed");
            let _ =
                WorkflowRun::update_status(&pool, &run_id, "failed", Some(&e.to_string())).await;
        }
    });
}

/// Execute the full DAG: parse → schedule → run nodes → finalize.
async fn execute_dag(
    pool: Arc<SqlitePool>,
    run_id: &str,
    yaml_content: &str,
) -> anyhow::Result<()> {
    let wf = crate::schema::parse_workflow(yaml_content)?;
    let nodes = &wf.nodes;

    // Mark run as started
    WorkflowRun::set_started(&pool, run_id).await?;

    // Topological sort
    let levels = topological_sort(nodes)?;

    // Build node_index map
    let node_index: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (node_id(n), i))
        .collect();

    // Concurrency limiter
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_NODES));

    // Track completed nodes
    let mut completed: HashSet<usize> = HashSet::new();
    let mut failed: HashSet<usize> = HashSet::new();

    for level in &levels {
        let mut tasks = Vec::new();

        for &idx in level {
            let node = &nodes[idx];
            let deps_ready = node_depends(node).iter().all(|dep| {
                node_index
                    .get(dep.as_str())
                    .map_or(false, |&di| completed.contains(&di))
            });

            if !deps_ready {
                tracing::warn!(node_id = %node_id(node), "dependencies not met, skipping");
                continue;
            }

            let pool = pool.clone();
            let semaphore = semaphore.clone();
            let run_id = run_id.to_string();
            let node = node.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                executor::execute_node(&pool, &run_id, &node).await
            });

            tasks.push((idx, task));
        }

        // Wait for all parallel nodes in this level
        for (idx, task) in tasks {
            match task.await {
                Ok(Ok(())) => {
                    completed.insert(idx);
                }
                Ok(Err(e)) => {
                    tracing::error!(node_idx = idx, error = %e, "node failed");
                    failed.insert(idx);
                }
                Err(e) => {
                    tracing::error!(node_idx = idx, error = %e, "node task panicked");
                    failed.insert(idx);
                }
            }
        }

        // Propagate failures: if any node failed and its downstream doesn't allow continue
        if !failed.is_empty() {
            for &fi in &failed {
                let node = &nodes[fi];
                if !node_continue_on_error(node) {
                    WorkflowRun::update_status(
                        &pool,
                        run_id,
                        "failed",
                        Some(&format!("node '{}' failed", node_id(node))),
                    )
                    .await?;
                    return Err(anyhow::anyhow!("node '{}' failed", node_id(node)));
                }
            }
        }
    }

    WorkflowRun::update_status(&pool, run_id, "success", None).await?;
    tracing::info!(run_id = %run_id, "workflow completed successfully");
    Ok(())
}

/// Topological sort returning levels of node indices that can run in parallel.
fn topological_sort(nodes: &[NodeDef]) -> anyhow::Result<Vec<Vec<usize>>> {
    let n = nodes.len();
    let id_to_idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (node_id(n), i))
        .collect();

    // Build adjacency + in-degree
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree = vec![0u32; n];

    for (i, node) in nodes.iter().enumerate() {
        for dep in node_depends(node) {
            let j = id_to_idx.get(dep.as_str()).with_context(|| {
                format!("node '{}' depends on unknown node '{}'", node_id(node), dep)
            })?;
            adj[*j].push(i);
            in_degree[i] += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut levels: Vec<Vec<usize>> = Vec::new();

    while !queue.is_empty() {
        let current_level: Vec<usize> = queue.drain(..).collect();
        levels.push(current_level.clone());

        let mut next_queue = VecDeque::new();
        for &node in &current_level {
            for &neighbor in &adj[node] {
                in_degree[neighbor] -= 1;
                if in_degree[neighbor] == 0 {
                    next_queue.push_back(neighbor);
                }
            }
        }
        queue = next_queue;
    }

    // Check for cycles
    if levels.iter().map(|l| l.len()).sum::<usize>() != n {
        anyhow::bail!("workflow contains a cycle");
    }

    Ok(levels)
}

// ─── Node Helpers ─────────────────────────────────────────────────

pub fn node_id(node: &NodeDef) -> &str {
    match node {
        NodeDef::Shell(n) => &n.id,
        NodeDef::Agent(n) => &n.id,
        NodeDef::Reference(n) => &n.id,
    }
}

fn node_depends(node: &NodeDef) -> &[String] {
    match node {
        NodeDef::Shell(n) => &n.depends,
        NodeDef::Agent(n) => &n.depends,
        NodeDef::Reference(n) => &n.depends,
    }
}

pub fn node_type_name(node: &NodeDef) -> &str {
    match node {
        NodeDef::Shell(_) => "shell",
        NodeDef::Agent(_) => "agent",
        NodeDef::Reference(_) => "reference",
    }
}

fn node_continue_on_error(node: &NodeDef) -> bool {
    match node {
        NodeDef::Shell(n) => n.continue_on_error,
        NodeDef::Agent(n) => n.continue_on_error,
        NodeDef::Reference(n) => n.continue_on_error,
    }
}
