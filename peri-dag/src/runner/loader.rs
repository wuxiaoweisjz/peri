use anyhow::Context;
use std::future::Future;
use std::pin::Pin;

use crate::schema::{parse_workflow, Workflow};

/// Load a workflow from a local path or HTTPS URL.
/// Resolves `references` recursively, inlining them into the node list.
pub async fn load_workflow(path_or_url: &str) -> anyhow::Result<Workflow> {
    load_workflow_inner(path_or_url).await
}

fn load_workflow_inner(
    path_or_url: &str,
) -> Pin<Box<dyn Future<Output = anyhow::Result<Workflow>> + Send + '_>> {
    Box::pin(async move {
        let content = fetch_content(path_or_url).await?;
        let mut wf = parse_workflow(&content)?;

        // Resolve reference nodes: for each Reference node, load the sub-workflow
        // and inline its nodes with prefixed IDs.
        if !wf.references.is_empty() {
            let mut inlined_nodes = Vec::new();

            for node in &wf.nodes {
                match node {
                    crate::schema::NodeDef::Reference(ref_node) => {
                        let ref_path = wf.references.get(&ref_node.r#ref).with_context(|| {
                            format!(
                                "reference '{}' not found in workflow references",
                                ref_node.r#ref
                            )
                        })?;

                        let sub_wf = load_workflow(ref_path).await?;
                        let prefix = format!("{}/", ref_node.id);

                        for sub_node in sub_wf.nodes {
                            let mut inlined = sub_node;
                            // Prefix IDs to avoid collisions
                            inlined = prefix_id(inlined, &prefix);
                            // Rewire depends: prefix node IDs
                            inlined = prefix_depends(inlined, &prefix);
                            inlined_nodes.push(inlined);
                        }
                    }
                    _ => inlined_nodes.push(node.clone()),
                }
            }

            wf.nodes = inlined_nodes;
        }

        Ok(wf)
    })
}

async fn fetch_content(path_or_url: &str) -> anyhow::Result<String> {
    if path_or_url.starts_with("https://") || path_or_url.starts_with("http://") {
        let client = reqwest::Client::new();
        let resp = client
            .get(path_or_url)
            .send()
            .await
            .with_context(|| format!("failed to fetch remote workflow: {path_or_url}"))?;
        let body = resp
            .text()
            .await
            .with_context(|| format!("failed to read response body from: {path_or_url}"))?;
        Ok(body)
    } else {
        std::fs::read_to_string(path_or_url)
            .with_context(|| format!("failed to read workflow file: {path_or_url}"))
    }
}

fn prefix_id(node: crate::schema::NodeDef, prefix: &str) -> crate::schema::NodeDef {
    use crate::schema::NodeDef;
    match node {
        NodeDef::Shell(mut n) => {
            n.id = format!("{prefix}{}", n.id);
            NodeDef::Shell(n)
        }
        NodeDef::Agent(mut n) => {
            n.id = format!("{prefix}{}", n.id);
            NodeDef::Agent(n)
        }
        NodeDef::Reference(mut n) => {
            n.id = format!("{prefix}{}", n.id);
            NodeDef::Reference(n)
        }
    }
}

fn prefix_depends(node: crate::schema::NodeDef, prefix: &str) -> crate::schema::NodeDef {
    use crate::schema::NodeDef;
    match node {
        NodeDef::Shell(mut n) => {
            n.depends = n
                .depends
                .into_iter()
                .map(|d| format!("{prefix}{d}"))
                .collect();
            NodeDef::Shell(n)
        }
        NodeDef::Agent(mut n) => {
            n.depends = n
                .depends
                .into_iter()
                .map(|d| format!("{prefix}{d}"))
                .collect();
            NodeDef::Agent(n)
        }
        NodeDef::Reference(mut n) => {
            n.depends = n
                .depends
                .into_iter()
                .map(|d| format!("{prefix}{d}"))
                .collect();
            NodeDef::Reference(n)
        }
    }
}
