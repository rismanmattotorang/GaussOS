// src/auth/mod.rs
//! Authentication and Authorization Module
//! Comprehensive security system with multiple authentication methods,
//! role-based access control, OAuth2/OIDC, MFA, and advanced security features

pub mod api_keys;
pub mod jwt;
pub mod mfa;
pub mod middleware;
pub mod oauth2;
pub mod permissions;
pub mod rate_limiter;
pub mod roles;
pub mod security_events;
pub mod session;

// Re-export main types for convenience
pub use api_keys::{ApiKey, ApiKeyManager, ApiKeyPermissions, CreateApiKeyRequest};
pub use jwt::{Claims, JwtConfig, JwtManager, TokenPair};
pub use mfa::{MfaChallenge, MfaManager, MfaMethod, TotpConfig};
pub use middleware::{AuthContext, AuthMiddleware, AuthenticationType, RateLimiter};
pub use oauth2::{OAuth2Config, OAuth2Manager, OAuth2Provider, OAuth2TokenResponse};
pub use permissions::{
    NamespacePermission, NamespacePermissionType, PermissionChecker, PermissionContext,
};
pub use roles::{CreateRoleRequest, Permission, PermissionCondition, Role, RoleManager};
pub use security_events::{
    SecurityEvent, SecurityEventLogger, SecurityEventType, SecuritySeverity,
};
pub use session::{CreateSessionRequest, Session, SessionData, SessionManager};

/// Simple AuthService wrapper for API authentication
/// This provides a convenient interface for basic authentication needs
pub struct AuthService {
    jwt_manager: JwtManager,
    jwt_secret: String,
}

impl AuthService {
    /// Create a new AuthService with the given JWT secret
    pub fn new(jwt_secret: String) -> Self {
        let jwt_config = JwtConfig {
            secret: jwt_secret.clone(),
            ..JwtConfig::default()
        };
        
        let jwt_manager = JwtManager::new(jwt_config)
            .expect("Failed to create JWT manager");
        
        Self {
            jwt_manager,
            jwt_secret,
        }
    }
    
    /// Validate a JWT token
    pub fn validate_token(&self, token: &str) -> crate::error::Result<Claims> {
        let validation = self.jwt_manager.validate_token(token);
        if validation.is_valid {
            validation.claims.ok_or_else(|| {
                crate::error::GaussOSError::AuthenticationError("Invalid token claims".to_string())
            })
        } else {
            Err(crate::error::GaussOSError::AuthenticationError(
                validation.error.unwrap_or_else(|| "Token validation failed".to_string())
            ))
        }
    }
    
    /// Generate a token pair for a user
    pub fn generate_tokens(&self, user_id: uuid::Uuid, username: &str, email: &str) -> crate::error::Result<TokenPair> {
        let claims = Claims::new(user_id, username, email);
        self.jwt_manager.generate_token_pair(&claims)
    }
    
    /// Refresh an access token using a refresh token
    pub fn refresh_token(&self, refresh_token: &str) -> crate::error::Result<TokenPair> {
        self.jwt_manager.refresh_access_token(refresh_token)
    }
    
    /// Check if a token is expired
    pub fn is_token_expired(&self, token: &str) -> bool {
        self.jwt_manager.is_token_expired(token)
    }
    
    /// Get the JWT secret (for internal use)
    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

use crate::{
    core::{MemCube, MemoryNamespace, MemoryPayload},
    database::{DatabaseVault, MemVault, SearchQuery},
    error::{GaussOSError, Result},
};
use chrono::{DateTime, Utc};
use md5;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

/// Enhanced authentication system with comprehensive security features
pub struct AuthSystem {
    pub jwt_manager: Arc<JwtManager>,
    pub api_key_manager: Arc<ApiKeyManager>,
    pub session_manager: Arc<SessionManager>,
    pub role_manager: Arc<RoleManager>,
    pub permission_checker: Arc<PermissionChecker>,
    pub rate_limiter: Arc<RateLimiter>,
    pub middleware: Arc<AuthMiddleware>,
    pub oauth2_manager: Arc<OAuth2Manager>,
    pub mfa_manager: Arc<MfaManager>,
    pub security_logger: Arc<SecurityEventLogger>,
    pub config: AuthSystemConfig,
}

/// Enhanced authentication system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSystemConfig {
    pub enable_oauth2: bool,
    pub enable_mfa: bool,
    pub enable_security_logging: bool,
    pub password_policy: PasswordPolicy,
    pub session_config: SessionConfig,
    pub security_config: SecurityConfig,
    pub oauth2_providers: Vec<OAuth2Provider>,
}

/// Enhanced user authentication data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub password_hash: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub failed_login_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
    pub password_changed_at: DateTime<Utc>,
    pub profile: UserProfile,
    pub security_settings: UserSecuritySettings,
    pub oauth2_connections: Vec<OAuth2Connection>,
    pub mfa_settings: Option<MfaSettings>,
}

/// Enhanced user profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub country: Option<String>,
    pub phone_number: Option<String>,
    pub phone_verified: bool,
    pub preferences: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// User security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSecuritySettings {
    pub require_mfa: bool,
    pub allowed_ip_ranges: Vec<String>,
    pub session_timeout_minutes: Option<u32>,
    pub require_password_change: bool,
    pub enable_login_notifications: bool,
    pub trusted_devices: Vec<TrustedDevice>,
}

/// OAuth2 connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Connection {
    pub provider: String,
    pub provider_user_id: String,
    pub connected_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// MFA settings for user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSettings {
    pub enabled: bool,
    pub primary_method: MfaMethod,
    pub backup_methods: Vec<MfaMethod>,
    pub recovery_codes: Vec<String>,
    pub last_used: Option<DateTime<Utc>>,
}

/// Trusted device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub id: Uuid,
    pub name: String,
    pub device_fingerprint: String,
    pub added_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub ip_address: String,
    pub user_agent: String,
}

/// Enhanced user creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub profile: UserProfile,
    pub require_email_verification: Option<bool>,
    pub initial_roles: Option<Vec<String>>,
}

impl CreateUserRequest {
    pub fn validate(&self) -> Result<()> {
        if self.username.len() < 3 || self.username.len() > 50 {
            return Err(GaussOSError::validation_failed("username".to_string(), "Length must be between 3 and 50 characters".to_string()));
        }
        if !self.email.contains('@') {
            return Err(GaussOSError::validation_failed("email".to_string(), "Invalid email format".to_string()));
        }
        if self.password.len() < 8 {
            return Err(GaussOSError::validation_failed("password".to_string(), "Password must be at least 8 characters".to_string()));
        }
        Ok(())
    }
}

/// Enhanced login request with device tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub identifier: String, // username or email
    pub password: String,
    pub remember_me: bool,
    pub device_info: Option<DeviceInfo>,
    pub mfa_token: Option<String>,
    pub trusted_device_token: Option<String>,
}

impl LoginRequest {
    pub fn validate(&self) -> Result<()> {
        if self.identifier.is_empty() {
            return Err(GaussOSError::validation_failed("identifier".to_string(), "Identifier cannot be empty".to_string()));
        }
        if self.password.is_empty() {
            return Err(GaussOSError::validation_failed("password".to_string(), "Password cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// Enhanced device information for security tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_type: String,
    pub os: String,
    pub os_version: String,
    pub browser: String,
    pub browser_version: String,
    pub ip_address: String,
    pub user_agent: String,
    pub fingerprint: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
}

/// Enhanced login response with comprehensive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub tokens: TokenPair,
    pub session_id: Uuid,
    pub permissions: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub mfa_required: bool,
    pub mfa_challenge: Option<MfaChallenge>,
    pub trusted_device: bool,
    pub security_notifications: Vec<SecurityNotification>,
}

/// Enhanced public user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub mfa_enabled: bool,
    pub oauth2_providers: Vec<String>,
}

/// Enhanced password requirements configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digits: bool,
    pub require_special_chars: bool,
    pub forbidden_patterns: Vec<String>,
    pub forbidden_passwords: Vec<String>,
    pub history_count: usize,
    pub max_age_days: Option<u32>,
    pub complexity_score_min: f64,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub default_timeout_minutes: u32,
    pub max_timeout_minutes: u32,
    pub extend_on_activity: bool,
    pub concurrent_sessions_limit: Option<u32>,
    pub require_fresh_login_for_sensitive: bool,
    pub track_ip_changes: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub max_failed_login_attempts: u32,
    pub account_lockout_duration_minutes: u32,
    pub enable_brute_force_protection: bool,
    pub enable_device_tracking: bool,
    pub enable_geolocation_tracking: bool,
    pub suspicious_activity_threshold: f64,
    pub enable_login_notifications: bool,
    pub require_secure_transport: bool,
}

/// Security notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityNotification {
    pub notification_type: SecurityNotificationType,
    pub message: String,
    pub severity: SecuritySeverity,
    pub timestamp: DateTime<Utc>,
    pub requires_action: bool,
}

/// Types of security notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityNotificationType {
    NewDeviceLogin,
    UnusualLocation,
    PasswordExpiring,
    SuspiciousActivity,
    MfaDisabled,
    AccountLocked,
    PermissionChanged,
}

impl AuthSystem {
    /// Create a new enhanced authentication system
    pub async fn new(database: Arc<dyn MemVault>, config: AuthSystemConfig) -> Result<Self> {
        // Initialize JWT manager
        let jwt_config = JwtConfig::default();
        let jwt_manager = Arc::new(JwtManager::new(jwt_config)?);

        // Initialize API key manager
        let api_key_manager = Arc::new(ApiKeyManager::new(database.clone()));

        // Initialize session manager with enhanced config
        let session_timeout = config.session_config.default_timeout_minutes as u64 * 60;
        let session_manager = Arc::new(SessionManager::new(
            database.clone(),
            session_timeout as i64,
        ));

        // Initialize role manager
        let role_manager = Arc::new(RoleManager::new(database.clone()));
        role_manager.initialize_system_roles().await?;

        // Initialize permission checker
        let permission_checker = Arc::new(PermissionChecker::new(role_manager.clone()));

        // Initialize rate limiter
        let rate_limiter = Arc::new(RateLimiter::new());

        // Initialize OAuth2 manager if enabled
        let oauth2_manager = if config.enable_oauth2 {
            Arc::new(OAuth2Manager::new(config.oauth2_providers.clone(), database.clone()).await?)
        } else {
            Arc::new(OAuth2Manager::disabled())
        };

        // Initialize MFA manager if enabled
        let mfa_manager = if config.enable_mfa {
            Arc::new(MfaManager::new(database.clone()).await?)
        } else {
            Arc::new(MfaManager::disabled())
        };

        // Initialize security event logger
        let security_logger = Arc::new(SecurityEventLogger::new(database.clone()));

        // Initialize middleware
        let middleware = Arc::new(AuthMiddleware::new(
            api_key_manager.clone(),
            session_manager.clone(),
            permission_checker.clone(),
            rate_limiter.clone(),
        ));

        Ok(Self {
            jwt_manager,
            api_key_manager,
            session_manager,
            role_manager,
            permission_checker,
            rate_limiter,
            middleware,
            oauth2_manager,
            mfa_manager,
            security_logger,
            config,
        })
    }

    /// Enhanced login with comprehensive security features
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        // Log login attempt
        self.security_logger
            .log_event(SecurityEvent {
                id: Uuid::new_v4(),
                event_type: SecurityEventType::Login,
                user_id: None,
                ip_address: request.device_info.as_ref().map(|d| d.ip_address.clone()),
                user_agent: request.device_info.as_ref().map(|d| d.user_agent.clone()),
                details: HashMap::new(),
                timestamp: Utc::now(),
                severity: SecuritySeverity::Low,
            })
            .await?;

        // Find user by identifier
        let user = self.find_user_by_identifier(&request.identifier).await?;

        // Check if account is locked
        if let Some(locked_until) = user.locked_until {
            if Utc::now() < locked_until {
                return Err(GaussOSError::AuthenticationError(format!(
                    "Account locked until {}",
                    locked_until
                )));
            }
        }

        // Verify password
        if !self.verify_password(&request.password, &user.password_hash)? {
            self.handle_failed_login(&user.id, request.device_info.as_ref())
                .await?;
            return Err(GaussOSError::AuthenticationFailed {
                reason: "Invalid credentials".to_string(),
                context: None,
            });
        }

        // Check if MFA is required
        let mfa_challenge = if user.mfa_settings.as_ref().map_or(false, |mfa| mfa.enabled) {
            if request.mfa_token.is_none() {
                // Generate MFA challenge
                let challenge = self.mfa_manager.generate_challenge(&user.id).await?;
                return Ok(LoginResponse {
                    user: self.user_to_user_info(&user),
                    tokens: TokenPair {
                        access_token: String::new(),
                        refresh_token: String::new(),
                        token_type: "Bearer".to_string(),
                        expires_in: 0,
                        scope: None,
                    },
                    session_id: Uuid::new_v4(),
                    permissions: Vec::new(),
                    expires_at: Utc::now(),
                    mfa_required: true,
                    mfa_challenge: Some(challenge),
                    trusted_device: false,
                    security_notifications: Vec::new(),
                });
            } else {
                // Verify MFA token
                let mfa_token = request.mfa_token.unwrap();
                if !self.mfa_manager.verify_token(&user.id, &mfa_token).await? {
                    return Err(GaussOSError::AuthenticationFailed {
                        reason: "Invalid MFA token".to_string(),
                        context: None,
                    });
                }
                None
            }
        } else {
            None
        };

        // Check device trust
        let trusted_device = self
            .check_trusted_device(&user, request.device_info.as_ref())
            .await?;

        // Generate tokens
        let claims = Claims::new(user.id, &user.username, &user.email);
        let tokens = self.jwt_manager.generate_token_pair(&claims)?;

        // Create token hashes for session storage (security best practice)
        let token_hash = format!("{:x}", md5::compute(&tokens.access_token));
        let refresh_token_hash = format!("{:x}", md5::compute(&tokens.refresh_token));

        // Create session data
        let session_data = SessionData {
            device_type: request.device_info.as_ref().map(|d| d.device_type.clone()),
            location: request
                .device_info
                .as_ref()
                .and_then(|d| d.timezone.clone()),
            is_mobile: request
                .device_info
                .as_ref()
                .map(|d| d.device_type.contains("mobile"))
                .unwrap_or(false),
            browser: request
                .device_info
                .as_ref()
                .map(|d| format!("{} {}", d.browser, d.browser_version)),
            os: request
                .device_info
                .as_ref()
                .map(|d| format!("{} {}", d.os, d.os_version)),
            metadata: std::collections::HashMap::new(),
        };

        // Create session
        let session_request = CreateSessionRequest {
            user_id: user.id,
            token_hash,
            refresh_token_hash: Some(refresh_token_hash),
            ip_address: request
                .device_info
                .as_ref()
                .and_then(|d| d.ip_address.parse().ok()),
            user_agent: request.device_info.as_ref().map(|d| d.user_agent.clone()),
            device_fingerprint: request
                .device_info
                .as_ref()
                .and_then(|d| d.fingerprint.clone()),
            session_data,
        };

        let session = self.session_manager.create_session(session_request).await?;

        // Get user permissions
        let permissions = self.get_user_permission_strings(&user.id).await?;

        // Reset failed login attempts
        self.reset_failed_login_attempts(&user.id).await?;

        // Update last login
        self.update_last_login(&user.id).await?;

        // Generate security notifications
        let security_notifications = self
            .generate_security_notifications(&user, request.device_info.as_ref())
            .await?;

        // Log successful login
        self.security_logger
            .log_event(SecurityEvent {
                id: Uuid::new_v4(),
                event_type: SecurityEventType::Login,
                user_id: Some(user.id),
                ip_address: request.device_info.as_ref().map(|d| d.ip_address.clone()),
                user_agent: request.device_info.as_ref().map(|d| d.user_agent.clone()),
                details: HashMap::new(),
                timestamp: Utc::now(),
                severity: SecuritySeverity::Low,
            })
            .await?;

        Ok(LoginResponse {
            user: self.user_to_user_info(&user),
            tokens,
            session_id: session.id,
            permissions,
            expires_at: session.expires_at,
            mfa_required: false,
            mfa_challenge,
            trusted_device,
            security_notifications,
        })
    }

    /// Enhanced user creation with comprehensive validation
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User> {
        // Validate password against policy
        self.validate_password_policy(&request.password)?;

        // Check if user already exists
        if self.user_exists(&request.username, &request.email).await? {
            return Err(GaussOSError::ConflictError(
                "User already exists".to_string(),
            ));
        }

        // Hash password
        let password_hash = self.hash_password(&request.password)?;

        let user = User {
            id: Uuid::new_v4(),
            username: request.username,
            email: request.email,
            email_verified: !request.require_email_verification.unwrap_or(true),
            password_hash,
            is_active: true,
            is_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            failed_login_attempts: 0,
            locked_until: None,
            password_changed_at: Utc::now(),
            profile: request.profile,
            security_settings: UserSecuritySettings {
                require_mfa: false,
                allowed_ip_ranges: Vec::new(),
                session_timeout_minutes: None,
                require_password_change: false,
                enable_login_notifications: true,
                trusted_devices: Vec::new(),
            },
            oauth2_connections: Vec::new(),
            mfa_settings: None,
        };

        // Store user
        self.store_user(&user).await?;

        // Assign initial roles if specified
        if let Some(roles) = request.initial_roles {
            for role_name in roles {
                if let Some(role) = self.role_manager.get_role_by_name(&role_name).await? {
                    self.role_manager
                        .assign_role(&user.id, &role.id, &user.id, None, Vec::new())
                        .await?;
                }
            }
        }

        // Log user creation
        self.security_logger
            .log_event(SecurityEvent {
                id: Uuid::new_v4(),
                event_type: SecurityEventType::Login, // TODO: Add UserCreated event type
                user_id: Some(user.id),
                ip_address: None,
                user_agent: None,
                details: HashMap::new(),
                timestamp: Utc::now(),
                severity: SecuritySeverity::Low,
            })
            .await?;

        Ok(user)
    }

    /// Enhanced password validation with comprehensive policy checks
    pub fn validate_password_policy(&self, password: &str) -> Result<()> {
        let policy = &self.config.password_policy;

        if password.len() < policy.min_length {
            return Err(GaussOSError::ValidationError(format!(
                "Password must be at least {} characters long",
                policy.min_length
            )));
        }

        if password.len() > policy.max_length {
            return Err(GaussOSError::ValidationError(format!(
                "Password must be no more than {} characters long",
                policy.max_length
            )));
        }

        if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(GaussOSError::ValidationError(
                "Password must contain at least one uppercase letter".to_string(),
            ));
        }

        if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(GaussOSError::ValidationError(
                "Password must contain at least one lowercase letter".to_string(),
            ));
        }

        if policy.require_digits && !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(GaussOSError::ValidationError(
                "Password must contain at least one digit".to_string(),
            ));
        }

        if policy.require_special_chars
            && !password
                .chars()
                .any(|c| "!@#$%^&*()_+-=[]{}|;':\",./<>?".contains(c))
        {
            return Err(GaussOSError::ValidationError(
                "Password must contain at least one special character".to_string(),
            ));
        }

        // Check forbidden patterns
        for pattern in &policy.forbidden_patterns {
            if password.to_lowercase().contains(&pattern.to_lowercase()) {
                return Err(GaussOSError::ValidationError(
                    "Password contains forbidden pattern".to_string(),
                ));
            }
        }

        // Check forbidden passwords
        if policy
            .forbidden_passwords
            .contains(&password.to_lowercase())
        {
            return Err(GaussOSError::ValidationError(
                "Password is in the list of forbidden passwords".to_string(),
            ));
        }

        // Calculate complexity score
        let complexity_score = self.calculate_password_complexity(password);
        if complexity_score < policy.complexity_score_min {
            return Err(GaussOSError::ValidationError(format!(
                "Password complexity score {:.2} is below minimum {:.2}",
                complexity_score, policy.complexity_score_min
            )));
        }

        Ok(())
    }

    fn calculate_password_complexity(&self, password: &str) -> f64 {
        let mut score = 0.0;

        // Length score
        score += (password.len() as f64 * 0.1).min(2.0);

        // Character variety score
        if password.chars().any(|c| c.is_lowercase()) {
            score += 1.0;
        }
        if password.chars().any(|c| c.is_uppercase()) {
            score += 1.0;
        }
        if password.chars().any(|c| c.is_ascii_digit()) {
            score += 1.0;
        }
        if password
            .chars()
            .any(|c| "!@#$%^&*()_+-=[]{}|;':\",./<>?".contains(c))
        {
            score += 1.0;
        }

        // Entropy score (simplified)
        let unique_chars = password
            .chars()
            .collect::<std::collections::HashSet<_>>()
            .len();
        score += (unique_chars as f64 * 0.1).min(1.0);

        score.min(10.0) // Max score of 10
    }

    async fn check_trusted_device(
        &self,
        user: &User,
        device_info: Option<&DeviceInfo>,
    ) -> Result<bool> {
        if let Some(device) = device_info {
            if let Some(fingerprint) = &device.fingerprint {
                return Ok(user
                    .security_settings
                    .trusted_devices
                    .iter()
                    .any(|trusted| trusted.device_fingerprint == *fingerprint));
            }
        }
        Ok(false)
    }

    async fn generate_security_notifications(
        &self,
        user: &User,
        device_info: Option<&DeviceInfo>,
    ) -> Result<Vec<SecurityNotification>> {
        let mut notifications = Vec::new();

        // Check for new device
        if let Some(device) = device_info {
            if !self.check_trusted_device(user, Some(device)).await? {
                notifications.push(SecurityNotification {
                    notification_type: SecurityNotificationType::NewDeviceLogin,
                    message: format!(
                        "New device login detected: {} on {}",
                        device.browser, device.os
                    ),
                    severity: SecuritySeverity::Medium,
                    timestamp: Utc::now(),
                    requires_action: false,
                });
            }
        }

        // Check for password expiration
        if let Some(max_age_days) = self.config.password_policy.max_age_days {
            let password_age = Utc::now() - user.password_changed_at;
            if password_age.num_days() > max_age_days as i64 - 7 {
                // 7 days warning
                notifications.push(SecurityNotification {
                    notification_type: SecurityNotificationType::PasswordExpiring,
                    message: "Your password will expire soon. Please change it.".to_string(),
                    severity: SecuritySeverity::Medium,
                    timestamp: Utc::now(),
                    requires_action: true,
                });
            }
        }

        Ok(notifications)
    }

    // ... (implement remaining methods similar to the original but with enhancements)

    async fn find_user_by_identifier(&self, identifier: &str) -> Result<User> {
        // TODO: Implement database lookup
        Err(GaussOSError::NotFound("User not found".to_string()))
    }

    async fn user_exists(&self, username: &str, email: &str) -> Result<bool> {
        // TODO: Implement database check
        Ok(false)
    }

    async fn store_user(&self, user: &User) -> Result<()> {
        // TODO: Implement database storage
        Ok(())
    }

    async fn handle_failed_login(
        &self,
        user_id: &Uuid,
        device_info: Option<&DeviceInfo>,
    ) -> Result<()> {
        // TODO: Implement failed login handling with lockout logic
        Ok(())
    }

    async fn reset_failed_login_attempts(&self, user_id: &Uuid) -> Result<()> {
        // TODO: Implement reset logic
        Ok(())
    }

    async fn update_last_login(&self, user_id: &Uuid) -> Result<()> {
        // TODO: Implement last login update
        Ok(())
    }

    async fn get_user_permission_strings(&self, user_id: &Uuid) -> Result<Vec<String>> {
        // TODO: Implement permission string retrieval
        Ok(vec!["read_memory".to_string(), "write_memory".to_string()])
    }

    fn user_to_user_info(&self, user: &User) -> UserInfo {
        UserInfo {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            email_verified: user.email_verified,
            display_name: user.profile.display_name.clone(),
            avatar_url: user.profile.avatar_url.clone(),
            is_active: user.is_active,
            last_login: user.last_login,
            created_at: user.created_at,
            roles: Vec::new(),       // TODO: Get actual roles
            permissions: Vec::new(), // TODO: Get actual permissions
            mfa_enabled: user.mfa_settings.as_ref().map_or(false, |mfa| mfa.enabled),
            oauth2_providers: user
                .oauth2_connections
                .iter()
                .map(|conn| conn.provider.clone())
                .collect(),
        }
    }

    fn hash_password(&self, password: &str) -> Result<String> {
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        use argon2::{Argon2, PasswordHasher};

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| {
                GaussOSError::system_error(
                    "auth".to_string(),
                    format!("Password hashing failed: {}", e),
                )
            })
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};

        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            GaussOSError::system_error("auth".to_string(), format!("Invalid password hash: {}", e))
        })?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

impl Default for AuthSystemConfig {
    fn default() -> Self {
        Self {
            enable_oauth2: true,
            enable_mfa: true,
            enable_security_logging: true,
            password_policy: PasswordPolicy::default(),
            session_config: SessionConfig::default(),
            security_config: SecurityConfig::default(),
            oauth2_providers: Vec::new(),
        }
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digits: true,
            require_special_chars: true,
            forbidden_patterns: vec![
                "password".to_string(),
                "123456".to_string(),
                "qwerty".to_string(),
            ],
            forbidden_passwords: Vec::new(),
            history_count: 5,
            max_age_days: Some(90),
            complexity_score_min: 6.0,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_timeout_minutes: 60,
            max_timeout_minutes: 480,
            extend_on_activity: true,
            concurrent_sessions_limit: Some(5),
            require_fresh_login_for_sensitive: true,
            track_ip_changes: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_failed_login_attempts: 5,
            account_lockout_duration_minutes: 30,
            enable_brute_force_protection: true,
            enable_device_tracking: true,
            enable_geolocation_tracking: false,
            suspicious_activity_threshold: 0.8,
            enable_login_notifications: true,
            require_secure_transport: true,
        }
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            first_name: None,
            last_name: None,
            avatar_url: None,
            timezone: None,
            language: None,
            country: None,
            phone_number: None,
            phone_verified: false,
            preferences: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}
