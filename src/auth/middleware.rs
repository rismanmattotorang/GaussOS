// src/auth/middleware.rs
//! Authentication Middleware
//! Provides comprehensive request authentication, rate limiting, and security features

use crate::{
    auth::{
        api_keys::ApiKeyManager,
        permissions::{PermissionChecker, PermissionContext},
        roles::Permission,
        session::SessionManager,
    },
    error::{GaussOSError, Result},
};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Authentication middleware state
#[derive(Clone)]
pub struct AuthMiddleware {
    pub api_key_manager: Arc<ApiKeyManager>,
    pub session_manager: Arc<SessionManager>,
    pub permission_checker: Arc<PermissionChecker>,
    pub rate_limiter: Arc<RateLimiter>,
}

/// Authentication context extracted from requests
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub auth_type: AuthenticationType,
    pub permissions: Vec<Permission>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: String,
    pub authenticated_at: DateTime<Utc>,
}

/// Types of authentication
#[derive(Debug, Clone, PartialEq)]
pub enum AuthenticationType {
    Session,
    ApiKey,
    Bearer,
    Basic,
}

/// Rate limiter for controlling request rates
pub struct RateLimiter {
    // In a real implementation, this would use Redis or similar
    limits: HashMap<String, RateLimit>,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub burst_limit: u32,
}

/// Rate limit result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_time: DateTime<Utc>,
    pub retry_after: Option<u64>,
}

/// Authentication error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthError {
    pub error_type: AuthErrorType,
    pub message: String,
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Types of authentication errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthErrorType {
    MissingCredentials,
    InvalidCredentials,
    ExpiredCredentials,
    InsufficientPermissions,
    RateLimitExceeded,
    AccountLocked,
    InvalidToken,
}

impl AuthMiddleware {
    /// Create new authentication middleware
    pub fn new(
        api_key_manager: Arc<ApiKeyManager>,
        session_manager: Arc<SessionManager>,
        permission_checker: Arc<PermissionChecker>,
        rate_limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            api_key_manager,
            session_manager,
            permission_checker,
            rate_limiter,
        }
    }

    /// Main authentication middleware handler
    pub async fn authenticate(
        State(auth): State<AuthMiddleware>,
        mut request: Request,
        next: Next,
    ) -> std::result::Result<Response, StatusCode> {
        let start_time = std::time::Instant::now();

        // Extract request metadata
        let headers = request.headers();
        let ip_address = extract_ip_address(headers);
        let user_agent = extract_user_agent(headers);
        let request_id = extract_or_generate_request_id(headers);

        // Check rate limits first
        if let Some(ip) = ip_address {
            let rate_limit_result = auth.rate_limiter.check_rate_limit(&ip.to_string()).await;
            if !rate_limit_result.allowed {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }

        // Try different authentication methods
        let auth_result = auth
            .try_authenticate(headers, ip_address, user_agent.clone(), &request_id)
            .await;

        match auth_result {
            Ok(auth_context) => {
                // Add auth context to request extensions
                request.extensions_mut().insert(auth_context);

                // Continue with the request
                let response = next.run(request).await;

                // Log successful authentication
                let duration = start_time.elapsed();
                tracing::info!(
                    "Authentication successful for request {} in {:?}",
                    request_id,
                    duration
                );

                Ok(response)
            }
            Err(error) => {
                // Log authentication failure
                tracing::warn!(
                    "Authentication failed for request {}: {:?}",
                    request_id,
                    error
                );

                match error {
                    GaussOSError::AuthenticationError(_) => Err(StatusCode::UNAUTHORIZED),
                    GaussOSError::AuthorizationError(_) => Err(StatusCode::FORBIDDEN),
                    GaussOSError::RateLimitError(_) => Err(StatusCode::TOO_MANY_REQUESTS),
                    _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
        }
    }

    /// Try different authentication methods
    async fn try_authenticate(
        &self,
        headers: &HeaderMap,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<String>,
        request_id: &str,
    ) -> Result<AuthContext> {
        // Try API key authentication
        if let Some(api_key) = extract_api_key(headers) {
            return self
                .authenticate_with_api_key(&api_key, ip_address, user_agent, request_id)
                .await;
        }

        // Try Bearer token authentication (JWT)
        if let Some(bearer_token) = extract_bearer_token(headers) {
            return self
                .authenticate_with_bearer_token(&bearer_token, ip_address, user_agent, request_id)
                .await;
        }

        // Try session authentication
        if let Some(session_token) = extract_session_token(headers) {
            return self
                .authenticate_with_session(&session_token, ip_address, user_agent, request_id)
                .await;
        }

        // Try basic authentication
        if let Some((username, password)) = extract_basic_auth(headers) {
            return self
                .authenticate_with_basic_auth(
                    &username, &password, ip_address, user_agent, request_id,
                )
                .await;
        }

        Err(GaussOSError::AuthenticationError(
            "No valid authentication credentials provided".to_string(),
        ))
    }

    /// Authenticate using API key
    async fn authenticate_with_api_key(
        &self,
        api_key: &str,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<String>,
        request_id: &str,
    ) -> Result<AuthContext> {
        let validation = self.api_key_manager.validate_api_key(api_key).await?;

        if !validation.is_valid {
            return Err(GaussOSError::AuthenticationError(
                validation
                    .error
                    .unwrap_or_else(|| "Invalid API key".to_string()),
            ));
        }

        let api_key_obj = validation.api_key.unwrap();

        Ok(AuthContext {
            user_id: api_key_obj.user_id,
            session_id: None,
            api_key_id: Some(api_key_obj.id),
            auth_type: AuthenticationType::ApiKey,
            permissions: vec![], // Will be loaded by permission checker
            ip_address,
            user_agent,
            request_id: request_id.to_string(),
            authenticated_at: Utc::now(),
        })
    }

    /// Authenticate using Bearer token (JWT)
    async fn authenticate_with_bearer_token(
        &self,
        token: &str,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<String>,
        request_id: &str,
    ) -> Result<AuthContext> {
        // TODO: Implement JWT validation using JwtManager
        // For now, return error
        Err(GaussOSError::AuthenticationError(
            "Bearer token authentication not yet implemented".to_string(),
        ))
    }

    /// Authenticate using session token
    async fn authenticate_with_session(
        &self,
        session_token: &str,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<String>,
        request_id: &str,
    ) -> Result<AuthContext> {
        // Hash the session token to find the session
        let session_hash = self.hash_session_token(session_token)?;

        if let Some(session) = self
            .session_manager
            .get_session_by_token(&session_hash)
            .await?
        {
            // Update last accessed time
            self.session_manager
                .update_session(
                    &session.id,
                    crate::auth::session::UpdateSessionRequest {
                        last_accessed: Some(Utc::now()),
                        ip_address,
                        user_agent: user_agent.clone(),
                        session_data: None,
                    },
                )
                .await?;

            Ok(AuthContext {
                user_id: session.user_id,
                session_id: Some(session.id),
                api_key_id: None,
                auth_type: AuthenticationType::Session,
                permissions: vec![], // Will be loaded by permission checker
                ip_address,
                user_agent,
                request_id: request_id.to_string(),
                authenticated_at: Utc::now(),
            })
        } else {
            Err(GaussOSError::AuthenticationError(
                "Invalid or expired session".to_string(),
            ))
        }
    }

    /// Authenticate using basic auth
    async fn authenticate_with_basic_auth(
        &self,
        username: &str,
        password: &str,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<String>,
        request_id: &str,
    ) -> Result<AuthContext> {
        // TODO: Implement basic authentication with user lookup
        // For now, return error
        Err(GaussOSError::AuthenticationError(
            "Basic authentication not yet implemented".to_string(),
        ))
    }

    /// Hash session token for lookup
    fn hash_session_token(&self, token: &str) -> Result<String> {
        // In a real implementation, use the same hashing method as session creation
        // For now, return the token as-is
        Ok(token.to_string())
    }
}

/// Permission middleware for checking specific permissions
pub async fn check_permission(
    State(auth): State<AuthMiddleware>,
    request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // Get auth context from request extensions
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract permission requirements from the request
    // This would typically be done based on the route or request attributes
    let permission_context = PermissionContext::new(
        auth_context.user_id,
        "memory", // This would be dynamic based on the endpoint
        "read",   // This would be dynamic based on the HTTP method
    )
    .with_ip_address(
        auth_context
            .ip_address
            .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
    )
    .with_request_id(&auth_context.request_id);

    // Check permission
    let permission_result = auth
        .permission_checker
        .check_permission(&permission_context)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if permission_result.allowed {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
        }
    }

    /// Check rate limit for a key (e.g., IP address, user ID)
    pub async fn check_rate_limit(&self, key: &str) -> RateLimitResult {
        // In a real implementation, this would use Redis or similar
        // For now, always allow requests
        RateLimitResult {
            allowed: true,
            remaining: 1000,
            reset_time: Utc::now() + chrono::Duration::minutes(1),
            retry_after: None,
        }
    }

    /// Configure rate limits for a key
    pub fn set_rate_limit(&mut self, key: &str, limit: RateLimit) {
        self.limits.insert(key.to_string(), limit);
    }

    /// Get rate limit configuration for a key
    pub fn get_rate_limit(&self, key: &str) -> Option<&RateLimit> {
        self.limits.get(key)
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            burst_limit: 10,
        }
    }
}

/// Extract IP address from request headers
fn extract_ip_address(headers: &HeaderMap) -> Option<std::net::IpAddr> {
    // Try X-Forwarded-For first (for proxies)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip_str) = forwarded_str.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return Some(ip);
            }
        }
    }

    None
}

/// Extract User-Agent from request headers
fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|ua| ua.to_str().ok())
        .map(|ua| ua.to_string())
}

/// Extract or generate request ID
fn extract_or_generate_request_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|id| id.to_str().ok())
        .map(|id| id.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Extract API key from headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    // Try X-API-Key header
    if let Some(api_key) = headers.get("x-api-key") {
        if let Ok(key_str) = api_key.to_str() {
            return Some(key_str.to_string());
        }
    }

    // Try Authorization header with API key format
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("ApiKey ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }

    None
}

/// Extract Bearer token from headers
fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }
    None
}

/// Extract session token from headers
fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    // Try X-Session-Token header
    if let Some(session) = headers.get("x-session-token") {
        if let Ok(session_str) = session.to_str() {
            return Some(session_str.to_string());
        }
    }

    // Try Cookie header for session token
    if let Some(cookie) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie.to_str() {
            for cookie_pair in cookie_str.split(';') {
                let cookie_pair = cookie_pair.trim();
                if cookie_pair.starts_with("session_token=") {
                    return Some(cookie_pair[14..].to_string());
                }
            }
        }
    }

    None
}

/// Extract basic auth credentials from headers
fn extract_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Basic ") {
                let encoded = &auth_str[6..];
                if let Ok(decoded) = general_purpose::STANDARD.decode(encoded) {
                    if let Ok(decoded_str) = String::from_utf8(decoded) {
                        if let Some(colon_pos) = decoded_str.find(':') {
                            let username = decoded_str[..colon_pos].to_string();
                            let password = decoded_str[colon_pos + 1..].to_string();
                            return Some((username, password));
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("test-api-key"));

        let api_key = extract_api_key(&headers);
        assert_eq!(api_key, Some("test-api-key".to_string()));
    }

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer test-token"),
        );

        let token = extract_bearer_token(&headers);
        assert_eq!(token, Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_ip_address() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("192.168.1.1, 10.0.0.1"),
        );

        let ip = extract_ip_address(&headers);
        assert_eq!(ip, Some("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn test_rate_limit_default() {
        let limit = RateLimit::default();
        assert_eq!(limit.requests_per_minute, 60);
        assert_eq!(limit.requests_per_hour, 1000);
        assert_eq!(limit.requests_per_day, 10000);
        assert_eq!(limit.burst_limit, 10);
    }
}
