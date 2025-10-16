//! HTTP Client Tool
//!
//! Information Hiding:
//! - HTTP client implementation details hidden
//! - Request/response handling abstracted
//! - Error handling and retries hidden

use super::{Tool, ToolMetadata, ToolParameter, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::time::{timeout, Duration};

/// HTTP request tool
pub struct HttpTool {
    client: Client,
    timeout_secs: u64,
    allowed_domains: Option<Vec<String>>,
}

impl HttpTool {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            client: Client::new(),
            timeout_secs,
            allowed_domains: None,
        }
    }

    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = Some(domains);
        self
    }

    /// Check if domain is allowed (internal security check)
    fn is_domain_allowed(&self, url: &str) -> bool {
        if let Some(ref allowed) = self.allowed_domains {
            allowed.iter().any(|domain| url.contains(domain))
        } else {
            true
        }
    }
}

#[async_trait]
impl Tool for HttpTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "http_request".to_string(),
            description: "Make HTTP GET or POST requests to fetch data from URLs.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "url".to_string(),
                    param_type: "string".to_string(),
                    description: "The URL to request".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "method".to_string(),
                    param_type: "string".to_string(),
                    description: "HTTP method (GET or POST), default is GET".to_string(),
                    required: false,
                },
                ToolParameter {
                    name: "body".to_string(),
                    param_type: "string".to_string(),
                    description: "Request body for POST requests".to_string(),
                    required: false,
                },
            ],
        }
    }

    fn validate(&self, args: &Value) -> Result<()> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'url' parameter is required and must be a string"))?;

        if url.is_empty() {
            return Err(anyhow::anyhow!("URL cannot be empty"));
        }

        if !self.is_domain_allowed(url) {
            return Err(anyhow::anyhow!(
                "Access to domain in '{}' is not allowed",
                url
            ));
        }

        // Validate HTTP method if provided
        if let Some(method) = args["method"].as_str() {
            let method_upper = method.to_uppercase();
            if method_upper != "GET" && method_upper != "POST" {
                return Err(anyhow::anyhow!("Only GET and POST methods are supported"));
            }
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        self.validate(&args)?;

        let url = args["url"].as_str().unwrap();
        let method = args["method"].as_str().unwrap_or("GET").to_uppercase();

        tracing::info!("Making HTTP {} request to: {}", method, url);

        let request_future = async {
            match method.as_str() {
                "GET" => {
                    let response = self.client.get(url).send().await?;
                    let status = response.status();
                    let body = response.text().await?;
                    Ok::<_, anyhow::Error>((status, body))
                }
                "POST" => {
                    let body_content = args["body"].as_str().unwrap_or("");
                    let response = self
                        .client
                        .post(url)
                        .body(body_content.to_string())
                        .send()
                        .await?;
                    let status = response.status();
                    let body = response.text().await?;
                    Ok::<_, anyhow::Error>((status, body))
                }
                _ => Err(anyhow::anyhow!("Unsupported method")),
            }
        };

        match timeout(Duration::from_secs(self.timeout_secs), request_future).await {
            Ok(Ok((status, body))) => {
                if status.is_success() {
                    Ok(ToolResult::success(format!(
                        "Status: {}\n\n{}",
                        status, body
                    )))
                } else {
                    Ok(ToolResult::failure(format!(
                        "HTTP error: {}\n\n{}",
                        status, body
                    )))
                }
            }
            Ok(Err(e)) => Ok(ToolResult::failure(format!("Request failed: {}", e))),
            Err(_) => Ok(ToolResult::failure(format!(
                "Request timed out after {} seconds",
                self.timeout_secs
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_http_get_request() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Start a mock HTTP server
        let mock_server = MockServer::start().await;

        // Configure the mock to return a successful response
        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Mock response"))
            .mount(&mock_server)
            .await;

        // Test HTTP GET with mock server
        let tool = HttpTool::new(10);
        let url = format!("{}/test", mock_server.uri());
        let args = json!({"url": url});

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Mock response"));
    }

    #[tokio::test]
    async fn test_http_domain_whitelist() {
        let tool = HttpTool::new(10).with_allowed_domains(vec!["httpbin.org".to_string()]);

        // Allowed domain - validation passes
        let args = json!({"url": "https://httpbin.org/get"});
        let validation = tool.validate(&args);
        assert!(validation.is_ok());

        // Disallowed domain - validation fails
        let args = json!({"url": "https://evil.com/steal-data"});
        let validation = tool.validate(&args);
        assert!(validation.is_err());
    }

    #[tokio::test]
    async fn test_http_metadata() {
        let tool = HttpTool::new(10);
        let metadata = tool.metadata();

        assert_eq!(metadata.name, "http_request");
        assert!(!metadata.description.is_empty());
        assert!(!metadata.parameters.is_empty());
    }
}
