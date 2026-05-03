use anyhow::Context;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::time::{timeout as tokio_timeout, Duration};
use uuid::Uuid;

use crate::db::NodeRun;
use crate::schema::{NodeDef, Platform};

/// Execute a single node and persist results to DB.
pub async fn execute_node(pool: &SqlitePool, run_id: &str, node: &NodeDef) -> anyhow::Result<()> {
    // Create node_run record
    let node_run = NodeRun {
        id: Uuid::now_v7().to_string(),
        run_id: run_id.to_string(),
        node_id: node_id(node).to_string(),
        node_type: node_type_name(node).to_string(),
        status: "pending".to_string(),
        attempt: 0,
        started_at: None,
        finished_at: None,
        exit_code: None,
        stdout: None,
        stderr: None,
        error_message: None,
    };
    node_run.insert(pool).await?;

    // Resolve script/prompt
    let platform = Platform::detect();

    match node {
        NodeDef::Shell(shell) => {
            let resolved = shell.run.resolve(platform);
            let script = load_script(&resolved)?;
            let env = build_env(&shell.env);
            run_shell(
                pool,
                &node_run.id,
                &script,
                &env,
                shell.exec.timeout,
                shell.exec.retry,
            )
            .await
        }
        NodeDef::Agent(agent) => {
            let resolved = agent.prompt.resolve(platform);
            let prompt = load_prompt(&resolved)?;
            let env = build_env(&agent.env);
            run_agent(
                pool,
                &node_run.id,
                &prompt,
                agent.model.as_deref(),
                agent.cwd.as_deref(),
                &env,
                agent.exec.timeout,
                agent.exec.retry,
            )
            .await
        }
        NodeDef::Reference(_reference) => {
            // Reference nodes are resolved at load time — they get inlined into the DAG.
            // For now this is a stub.
            NodeRun::update_result(
                pool,
                &node_run.id,
                "success",
                Some(0),
                Some("reference node (not yet implemented)"),
                None,
                None,
            )
            .await?;
            Ok(())
        }
    }
}

// ─── Shell Execution ──────────────────────────────────────────────

async fn run_shell(
    pool: &SqlitePool,
    node_run_id: &str,
    script: &str,
    env: &HashMap<String, String>,
    timeout_secs: Option<u64>,
    retries: Option<u32>,
) -> anyhow::Result<()> {
    let max_attempts = retries.unwrap_or(0) + 1;
    let mut last_error = None;

    for attempt in 0..max_attempts {
        // Update attempt
        sqlx::query("UPDATE node_runs SET attempt = ? WHERE id = ?")
            .bind(attempt as i64)
            .bind(node_run_id)
            .execute(pool)
            .await?;

        NodeRun::set_started(pool, node_run_id).await?;

        let result = execute_shell_command(script, env, timeout_secs).await;

        match result {
            Ok((exit_code, stdout, stderr)) => {
                let status = if exit_code == 0 { "success" } else { "failed" };
                NodeRun::update_result(
                    pool,
                    node_run_id,
                    status,
                    Some(exit_code),
                    Some(&stdout),
                    Some(&stderr),
                    None,
                )
                .await?;

                if exit_code == 0 {
                    return Ok(());
                }
                last_error = Some(anyhow::anyhow!(
                    "shell exited with code {exit_code}\nstderr: {stderr}"
                ));
            }
            Err(e) => {
                last_error = Some(e);
                // exponential backoff between retries
                tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            }
        }
    }

    let err = last_error.unwrap_or_else(|| anyhow::anyhow!("shell execution failed"));
    NodeRun::update_result(
        pool,
        node_run_id,
        "failed",
        None,
        None,
        None,
        Some(&err.to_string()),
    )
    .await?;
    Err(err)
}

async fn execute_shell_command(
    script: &str,
    env: &HashMap<String, String>,
    timeout_secs: Option<u64>,
) -> anyhow::Result<(i64, String, String)> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.arg("/C");
        c
    } else {
        let mut c = Command::new("bash");
        c.arg("-c");
        c
    };

    cmd.arg(script);
    cmd.envs(env);
    cmd.kill_on_drop(true);

    let output = if let Some(secs) = timeout_secs {
        tokio_timeout(Duration::from_secs(secs), cmd.output())
            .await
            .context("shell command timed out")??
    } else {
        cmd.output().await?
    };

    let exit_code = output.status.code().unwrap_or(-1) as i64;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok((exit_code, stdout, stderr))
}

// ─── Agent Execution ──────────────────────────────────────────────

async fn run_agent(
    pool: &SqlitePool,
    node_run_id: &str,
    prompt: &str,
    model: Option<&str>,
    cwd: Option<&str>,
    env: &HashMap<String, String>,
    timeout_secs: Option<u64>,
    retries: Option<u32>,
) -> anyhow::Result<()> {
    let max_attempts = retries.unwrap_or(0) + 1;
    let mut last_error = None;

    for attempt in 0..max_attempts {
        sqlx::query("UPDATE node_runs SET attempt = ? WHERE id = ?")
            .bind(attempt as i64)
            .bind(node_run_id)
            .execute(pool)
            .await?;

        NodeRun::set_started(pool, node_run_id).await?;

        let result = execute_agent_command(prompt, model, cwd, env, timeout_secs).await;

        match result {
            Ok((exit_code, stdout, stderr)) => {
                let status = if exit_code == 0 { "success" } else { "failed" };
                NodeRun::update_result(
                    pool,
                    node_run_id,
                    status,
                    Some(exit_code),
                    Some(&stdout),
                    Some(&stderr),
                    None,
                )
                .await?;

                if exit_code == 0 {
                    return Ok(());
                }
                last_error = Some(anyhow::anyhow!(
                    "agent exited with code {exit_code}\nstderr: {stderr}"
                ));
            }
            Err(e) => {
                last_error = Some(e);
                tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            }
        }
    }

    let err = last_error.unwrap_or_else(|| anyhow::anyhow!("agent execution failed"));
    NodeRun::update_result(
        pool,
        node_run_id,
        "failed",
        None,
        None,
        None,
        Some(&err.to_string()),
    )
    .await?;
    Err(err)
}

async fn execute_agent_command(
    prompt: &str,
    model: Option<&str>,
    cwd: Option<&str>,
    env: &HashMap<String, String>,
    timeout_secs: Option<u64>,
) -> anyhow::Result<(i64, String, String)> {
    let mut cmd = Command::new("acpx");
    cmd.arg("run").arg("--prompt").arg(prompt);

    if let Some(model) = model {
        cmd.arg("--model").arg(model);
    }
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    cmd.envs(env);
    cmd.kill_on_drop(true);

    let output = if let Some(secs) = timeout_secs {
        tokio_timeout(Duration::from_secs(secs), cmd.output())
            .await
            .context("agent command timed out")??
    } else {
        cmd.output().await?
    };

    let exit_code = output.status.code().unwrap_or(-1) as i64;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok((exit_code, stdout, stderr))
}

// ─── Helpers ──────────────────────────────────────────────────────

fn load_script(resolved: &crate::schema::ResolvedScript) -> anyhow::Result<String> {
    match resolved {
        crate::schema::ResolvedScript::Inline(s) => Ok(s.clone()),
        crate::schema::ResolvedScript::File(path) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read script file: {path}")),
    }
}

fn load_prompt(resolved: &crate::schema::ResolvedPrompt) -> anyhow::Result<String> {
    match resolved {
        crate::schema::ResolvedPrompt::Inline(s) => Ok(s.clone()),
        crate::schema::ResolvedPrompt::File(path) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read prompt file: {path}")),
    }
}

fn build_env(node_env: &HashMap<String, String>) -> HashMap<String, String> {
    let mut env = HashMap::new();
    // Inherit current process env
    for (k, v) in std::env::vars() {
        env.insert(k, v);
    }
    // Override with node-specific env
    for (k, v) in node_env {
        env.insert(k.clone(), v.clone());
    }
    env
}

// Duplicated from mod.rs for executor.rs to use locally (avoid circular dep)
fn node_id(node: &NodeDef) -> &str {
    match node {
        NodeDef::Shell(n) => &n.id,
        NodeDef::Agent(n) => &n.id,
        NodeDef::Reference(n) => &n.id,
    }
}

fn node_type_name(node: &NodeDef) -> &str {
    match node {
        NodeDef::Shell(_) => "shell",
        NodeDef::Agent(_) => "agent",
        NodeDef::Reference(_) => "reference",
    }
}
