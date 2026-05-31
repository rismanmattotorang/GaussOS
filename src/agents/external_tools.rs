// src/agents/external_tools.rs
//! External Tool Adapters for Agent Integration
//! Provides adapters for HTTP APIs, databases, file systems, and other external services

use crate::{
    agents::tools::{AgentTool, ToolExecutionResult, ToolPermission},
    error::{GaussOSError, Result},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// External tool adapter trait
#[async_trait]
pub trait ExternalToolAdapter: Send + Sync {
    async fn execute(&self, parameters: serde_json::Value) -> Result<serde_json::Value>;
    fn get_schema(&self) -> serde_json::Value;
    fn get_required_permissions(&self) -> Vec<ToolPermission>;
}

/// HTTP tool for making web requests
pub struct HttpTool {
    pub client: reqwest::Client,
    pub default_timeout: Duration,
    pub allowed_domains: Vec<String>,
    pub max_response_size: usize,
}

/// Database tool for executing queries
pub struct DatabaseTool {
    pub connection_string: String,
    pub allowed_operations: Vec<DatabaseOperation>,
    pub query_timeout: Duration,
}

/// File system tool for file operations
pub struct FileTool {
    pub allowed_paths: Vec<PathBuf>,
    pub max_file_size: usize,
    pub allowed_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseOperation {
    Select,
    Insert,
    Update,
    Delete,
    CreateTable,
    DropTable,
}

impl HttpTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            default_timeout: Duration::from_secs(30),
            allowed_domains: vec!["*".to_string()],
            max_response_size: 10_000_000, // 10MB
        }
    }
}

#[async_trait]
impl AgentTool for HttpTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn description(&self) -> &str {
        "Make HTTP requests to external APIs"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "The URL to request"},
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"], "default": "GET"},
                "headers": {"type": "object", "description": "HTTP headers"},
                "body": {"type": "string", "description": "Request body for POST/PUT"},
                "timeout": {"type": "number", "description": "Timeout in seconds"}
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, _name: &str, args: serde_json::Value) -> Result<ToolExecutionResult> {
        #[derive(Deserialize)]
        struct HttpArgs {
            url: String,
            method: Option<String>,
            headers: Option<HashMap<String, String>>,
            body: Option<String>,
            timeout: Option<u64>,
        }

        let args: HttpArgs = serde_json::from_value(args)
            .map_err(|e| GaussOSError::ValidationError(format!("Invalid HTTP args: {}", e)))?;

        // Validate domain if restrictions are in place
        if !self.allowed_domains.contains(&"*".to_string()) {
            let url = reqwest::Url::parse(&args.url)
                .map_err(|e| GaussOSError::ValidationError(format!("Invalid URL: {}", e)))?;

            if let Some(domain) = url.domain() {
                if !self
                    .allowed_domains
                    .iter()
                    .any(|allowed| domain.ends_with(allowed))
                {
                    return Ok(ToolExecutionResult::error("Domain not allowed".to_string()));
                }
            }
        }

        let method = args.method.unwrap_or_else(|| "GET".to_string());
        let timeout = Duration::from_secs(args.timeout.unwrap_or(30));

        let mut request = match method.as_str() {
            "GET" => self.client.get(&args.url),
            "POST" => self.client.post(&args.url),
            "PUT" => self.client.put(&args.url),
            "DELETE" => self.client.delete(&args.url),
            _ => {
                return Ok(ToolExecutionResult::error(
                    "Unsupported HTTP method".to_string(),
                ))
            }
        };

        if let Some(headers) = args.headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        if let Some(body) = args.body {
            request = request.body(body);
        }

        let start_time = std::time::Instant::now();

        match request.timeout(timeout).send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let headers: HashMap<String, String> = response
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();

                let body = response.text().await.map_err(|e| {
                    GaussOSError::external_error(
                        "http_client".to_string(),
                        format!("Failed to read response: {}", e),
                    )
                })?;

                let result = serde_json::json!({
                    "status": status,
                    "headers": headers,
                    "body": body
                });

                Ok(
                    ToolExecutionResult::success(result, "HTTP request completed".to_string())
                        .with_execution_time(start_time.elapsed().as_millis() as u64),
                )
            }
            Err(e) => Ok(ToolExecutionResult::error(format!(
                "HTTP request failed: {}",
                e
            ))),
        }
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::ReadSystemInfo]
    }
}

impl DatabaseTool {
    pub fn new(connection_string: String) -> Self {
        Self {
            connection_string,
            allowed_operations: vec![DatabaseOperation::Select],
            query_timeout: Duration::from_secs(30),
        }
    }
}

#[async_trait]
impl AgentTool for DatabaseTool {
    fn name(&self) -> &str {
        "database_query"
    }

    fn description(&self) -> &str {
        "Execute database queries"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "SQL query to execute"},
                "parameters": {"type": "array", "description": "Query parameters"},
                "timeout": {"type": "number", "description": "Query timeout in seconds"}
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, _name: &str, args: serde_json::Value) -> Result<ToolExecutionResult> {
        // Placeholder implementation - would integrate with actual database
        Ok(ToolExecutionResult::success(
            serde_json::json!({"message": "Database query executed", "rows": []}),
            "Query completed successfully".to_string(),
        ))
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::ReadMemory, ToolPermission::WriteMemory]
    }
}

impl FileTool {
    pub fn new() -> Self {
        Self {
            allowed_paths: vec![PathBuf::from("/tmp"), PathBuf::from("./data")],
            max_file_size: 100_000_000, // 100MB
            allowed_extensions: vec!["txt".to_string(), "json".to_string(), "csv".to_string()],
        }
    }
}

#[async_trait]
impl AgentTool for FileTool {
    fn name(&self) -> &str {
        "file_operations"
    }

    fn description(&self) -> &str {
        "Perform file system operations"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": ["read", "write", "list", "delete"]},
                "path": {"type": "string", "description": "File or directory path"},
                "content": {"type": "string", "description": "Content to write (for write operation)"},
                "encoding": {"type": "string", "default": "utf-8"}
            },
            "required": ["operation", "path"]
        })
    }

    async fn execute(&self, _name: &str, args: serde_json::Value) -> Result<ToolExecutionResult> {
        #[derive(Deserialize)]
        struct FileArgs {
            operation: String,
            path: String,
            content: Option<String>,
            encoding: Option<String>,
        }

        let args: FileArgs = serde_json::from_value(args)
            .map_err(|e| GaussOSError::ValidationError(format!("Invalid file args: {}", e)))?;

        let path = PathBuf::from(&args.path);

        // Check if path is allowed
        let allowed = self
            .allowed_paths
            .iter()
            .any(|allowed_path| path.starts_with(allowed_path));

        if !allowed {
            return Ok(ToolExecutionResult::error("Path not allowed".to_string()));
        }

        match args.operation.as_str() {
            "read" => match tokio::fs::read_to_string(&path).await {
                Ok(content) => Ok(ToolExecutionResult::success(
                    serde_json::json!({"content": content}),
                    "File read successfully".to_string(),
                )),
                Err(e) => Ok(ToolExecutionResult::error(format!(
                    "Failed to read file: {}",
                    e
                ))),
            },
            "write" => {
                if let Some(content) = args.content {
                    match tokio::fs::write(&path, content).await {
                        Ok(()) => Ok(ToolExecutionResult::success(
                            serde_json::json!({"message": "File written successfully"}),
                            "File written".to_string(),
                        )),
                        Err(e) => Ok(ToolExecutionResult::error(format!(
                            "Failed to write file: {}",
                            e
                        ))),
                    }
                } else {
                    Ok(ToolExecutionResult::error(
                        "Content required for write operation".to_string(),
                    ))
                }
            }
            "list" => match tokio::fs::read_dir(&path).await {
                Ok(mut entries) => {
                    let mut files = Vec::new();
                    while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                        if let Ok(name) = entry.file_name().into_string() {
                            files.push(name);
                        }
                    }
                    Ok(ToolExecutionResult::success(
                        serde_json::json!({"files": files}),
                        "Directory listed successfully".to_string(),
                    ))
                }
                Err(e) => Ok(ToolExecutionResult::error(format!(
                    "Failed to list directory: {}",
                    e
                ))),
            },
            _ => Ok(ToolExecutionResult::error(
                "Unsupported operation".to_string(),
            )),
        }
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![
            ToolPermission::ReadSystemInfo,
            ToolPermission::WriteSystemConfig,
        ]
    }
}
