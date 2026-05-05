use async_trait::async_trait;
use rust_create_agent::tools::BaseTool;
use serde_json::Value;
use std::net::IpAddr;
use tokio::time::{timeout, Duration};
use url::Url;

/// URL 安全校验，防止 SSRF
fn validate_url(url: &str) -> Result<Url, String> {
    let parsed = Url::parse(url).map_err(|e| format!("无效的 URL: {e}"))?;

    // 仅允许 http/https
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("仅支持 http/https 协议".to_string()),
    }

    // 检查主机名
    match parsed.host() {
        None => return Err("URL 缺少主机名".to_string()),
        Some(url::Host::Domain(_)) => {
            // 域名不做 DNS 解析，直接通过
        }
        Some(url::Host::Ipv4(ip)) => {
            if ip.is_loopback() {
                return Err("禁止访问回环地址".to_string());
            }
            if ip.is_private() {
                return Err("禁止访问私有地址".to_string());
            }
            if ip.is_link_local() {
                return Err("禁止访问链路本地地址".to_string());
            }
            if ip == IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED) {
                return Err("禁止访问未指定地址".to_string());
            }
        }
        Some(url::Host::Ipv6(ip)) => {
            if ip.is_loopback() {
                return Err("禁止访问回环地址".to_string());
            }
            if ip.is_unicast_link_local() {
                return Err("禁止访问链路本地地址".to_string());
            }
            // IPv6 私有地址：fc00::/7 (unique local)
            let segments = ip.segments();
            if (segments[0] & 0xfe00) == 0xfc00 {
                return Err("禁止访问私有地址".to_string());
            }
            if ip == IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED) {
                return Err("禁止访问未指定地址".to_string());
            }
        }
    }

    Ok(parsed)
}

/// HTML 转纯文本（120 列宽度）
fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 120).unwrap_or_default()
}

/// 按行数截断内容
fn truncate_content(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= max_lines {
        content.to_string()
    } else {
        let truncated: String = lines[..max_lines].join("\n");
        format!("{truncated}\n[内容已截断，原始内容共 {} 行]", lines.len())
    }
}

/// WebFetch 工具 — 抓取 URL 并返回文本内容
pub struct WebFetchTool;

const WEB_FETCH_DESCRIPTION: &str = r#"Fetches a web page by URL and returns its content as text.

Usage:
- Only http:// and https:// URLs are allowed
- HTML pages are converted to readable text; JSON is pretty-printed; plain text is returned as-is
- Binary content returns only type and size information
- Results are truncated at 2000 lines
- An optional 'prompt' parameter provides guidance for how to use the fetched content

Security:
- Internal/private/loopback IP addresses are blocked
- Maximum response size: 10MB
- Request timeout: 30 seconds
- Maximum redirects: 5"#;

/// 响应体大小上限
const MAX_RESPONSE_BYTES: u64 = 10 * 1024 * 1024; // 10MB

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BaseTool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> &str {
        WEB_FETCH_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要抓取的完整 URL（http/https）"
                },
                "prompt": {
                    "type": "string",
                    "description": "可选。提取内容的指导提示，附在结果前供 LLM 参考"
                }
            },
            "required": ["url"]
        })
    }

    async fn invoke(
        &self,
        input: Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = input["url"].as_str().ok_or("Missing url parameter")?;
        let prompt = input["prompt"].as_str();

        let parsed_url = validate_url(url)?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent("perihelion/1.0")
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

        let resp = client
            .get(parsed_url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        // 检查响应体大小
        if let Some(len) = resp.content_length() {
            if len > MAX_RESPONSE_BYTES {
                return Ok(format!("响应体超过 10MB 限制（{len} bytes）"));
            }
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // 读取 body（带超时）
        let body = timeout(Duration::from_secs(30), resp.text())
            .await
            .map_err(|_| "读取响应体超时（30秒）")?
            .map_err(|e| format!("读取响应体失败: {e}"))?;

        // 实际大小检查（当 content-length 不可用时）
        if body.len() as u64 > MAX_RESPONSE_BYTES {
            return Ok(format!("响应体超过 10MB 限制（{} bytes）", body.len()));
        }

        let processed = if content_type.contains("text/html") {
            html_to_text(&body)
        } else if content_type.contains("text/plain") {
            body
        } else if content_type.contains("application/json") {
            match serde_json::from_str::<Value>(&body) {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(body),
                Err(_) => body,
            }
        } else {
            format!(
                "Content-Type: {content_type}\nSize: {} bytes\n（不支持的内容类型）",
                body.len()
            )
        };

        let truncated = truncate_content(&processed, 2000);

        let result = match prompt {
            Some(p) => format!("提示: {p}\n\n{truncated}"),
            None => truncated,
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_rejects_ftp() {
        let err = validate_url("ftp://example.com").unwrap_err();
        assert!(err.contains("仅支持 http/https"), "实际: {err}");
    }

    #[test]
    fn test_validate_url_rejects_localhost() {
        let err = validate_url("http://127.0.0.1/test").unwrap_err();
        assert!(err.contains("回环地址"), "实际: {err}");
    }

    #[test]
    fn test_validate_url_rejects_private_ip() {
        let err = validate_url("http://192.168.1.1/test").unwrap_err();
        assert!(err.contains("私有地址"), "实际: {err}");
    }

    #[test]
    fn test_validate_url_rejects_link_local() {
        let err = validate_url("http://169.254.1.1/test").unwrap_err();
        assert!(err.contains("链路本地"), "实际: {err}");
    }

    #[test]
    fn test_validate_url_accepts_https() {
        assert!(validate_url("https://example.com/page").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_invalid_url() {
        let err = validate_url("not-a-url").unwrap_err();
        assert!(err.contains("无效的 URL"), "实际: {err}");
    }

    #[test]
    fn test_truncate_content_no_truncation() {
        let lines: Vec<String> = (0..10).map(|i| format!("line {i}")).collect();
        let input = lines.join("\n");
        assert_eq!(truncate_content(&input, 2000), input);
    }

    #[test]
    fn test_truncate_content_with_truncation() {
        let lines: Vec<String> = (0..3000).map(|i| format!("line {i}")).collect();
        let input = lines.join("\n");
        let result = truncate_content(&input, 2000);
        assert!(result.contains("[内容已截断，原始内容共 3000 行]"));
        assert!(result.contains("line 0"));
        assert!(result.contains("line 1999"));
        assert!(!result.contains("line 2000"));
    }

    #[test]
    fn test_html_to_text_basic() {
        let result = html_to_text("<p>Hello</p>");
        assert!(result.contains("Hello"), "实际: {result}");
    }

    #[test]
    fn test_tool_name_is_web_fetch() {
        assert_eq!(WebFetchTool::new().name(), "WebFetch");
    }

    #[test]
    fn test_tool_parameters_required_url() {
        let params = WebFetchTool::new().parameters();
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&Value::String("url".to_string())));
    }
}
