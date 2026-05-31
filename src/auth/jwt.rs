// src/auth/jwt.rs
//! JWT Token Management
//! Provides secure JWT token generation, validation, and management

use crate::error::{GaussOSError, Result};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Manager for handling token operations
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    access_token_expiry: Duration,
    refresh_token_expiry: Duration,
    issuer: String,
    audience: Vec<String>,
}

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub algorithm: Algorithm,
    pub access_token_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
    pub issuer: String,
    pub audience: Vec<String>,
}

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// Email
    pub email: String,
    /// Issued at
    pub iat: i64,
    /// Expiration time
    pub exp: i64,
    /// Not before
    pub nbf: i64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: Vec<String>,
    /// JWT ID
    pub jti: String,
    /// Token type (access or refresh)
    pub token_type: TokenType,
    /// User roles
    pub roles: Vec<String>,
    /// Custom claims
    pub custom: serde_json::Value,
}

/// Token type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Access,
    Refresh,
}

/// Token pair containing access and refresh tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: Option<String>,
}

/// Token validation result
#[derive(Debug, Clone)]
pub struct TokenValidation {
    pub is_valid: bool,
    pub claims: Option<Claims>,
    pub error: Option<String>,
}

impl JwtManager {
    /// Create a new JWT manager
    pub fn new(config: JwtConfig) -> Result<Self> {
        let encoding_key = EncodingKey::from_secret(config.secret.as_ref());
        let decoding_key = DecodingKey::from_secret(config.secret.as_ref());

        Ok(Self {
            encoding_key,
            decoding_key,
            algorithm: config.algorithm,
            access_token_expiry: Duration::hours(config.access_token_expiry_hours),
            refresh_token_expiry: Duration::days(config.refresh_token_expiry_days),
            issuer: config.issuer,
            audience: config.audience,
        })
    }

    /// Generate access token
    pub fn generate_access_token(&self, claims: &Claims) -> Result<String> {
        let mut token_claims = claims.clone();
        token_claims.token_type = TokenType::Access;
        token_claims.exp = (Utc::now() + self.access_token_expiry).timestamp();
        token_claims.iat = Utc::now().timestamp();
        token_claims.nbf = Utc::now().timestamp();
        token_claims.jti = Uuid::new_v4().to_string();
        token_claims.iss = self.issuer.clone();
        token_claims.aud = self.audience.clone();

        let header = Header::new(self.algorithm);
        encode(&header, &token_claims, &self.encoding_key).map_err(|e| {
            GaussOSError::AuthenticationError(format!("Failed to generate access token: {}", e))
        })
    }

    /// Generate refresh token
    pub fn generate_refresh_token(&self, claims: &Claims) -> Result<String> {
        let mut token_claims = claims.clone();
        token_claims.token_type = TokenType::Refresh;
        token_claims.exp = (Utc::now() + self.refresh_token_expiry).timestamp();
        token_claims.iat = Utc::now().timestamp();
        token_claims.nbf = Utc::now().timestamp();
        token_claims.jti = Uuid::new_v4().to_string();
        token_claims.iss = self.issuer.clone();
        token_claims.aud = self.audience.clone();

        let header = Header::new(self.algorithm);
        encode(&header, &token_claims, &self.encoding_key).map_err(|e| {
            GaussOSError::AuthenticationError(format!("Failed to generate refresh token: {}", e))
        })
    }

    /// Generate token pair (access + refresh)
    pub fn generate_token_pair(&self, claims: &Claims) -> Result<TokenPair> {
        let access_token = self.generate_access_token(claims)?;
        let refresh_token = self.generate_refresh_token(claims)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_expiry.num_seconds(),
            scope: None,
        })
    }

    /// Validate and decode token
    pub fn validate_token(&self, token: &str) -> TokenValidation {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&self.audience);

        match decode::<Claims>(token, &self.decoding_key, &validation) {
            Ok(token_data) => {
                // Additional validation checks
                if let Err(error) = self.additional_validation(&token_data.claims) {
                    TokenValidation {
                        is_valid: false,
                        claims: None,
                        error: Some(error),
                    }
                } else {
                    TokenValidation {
                        is_valid: true,
                        claims: Some(token_data.claims),
                        error: None,
                    }
                }
            }
            Err(e) => TokenValidation {
                is_valid: false,
                claims: None,
                error: Some(format!("Token validation failed: {}", e)),
            },
        }
    }

    /// Refresh access token using refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair> {
        let validation_result = self.validate_token(refresh_token);

        if !validation_result.is_valid {
            return Err(GaussOSError::AuthenticationError(
                validation_result
                    .error
                    .unwrap_or_else(|| "Invalid refresh token".to_string()),
            ));
        }

        let claims = validation_result.claims.unwrap();

        // Ensure it's a refresh token
        if claims.token_type != TokenType::Refresh {
            return Err(GaussOSError::AuthenticationError(
                "Token is not a refresh token".to_string(),
            ));
        }

        // Generate new token pair
        self.generate_token_pair(&claims)
    }

    /// Extract claims from token without validation (for inspection)
    pub fn extract_claims(&self, token: &str) -> Result<Claims> {
        let validation = Validation::new(self.algorithm);
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation).map_err(|e| {
            GaussOSError::AuthenticationError(format!("Failed to extract claims: {}", e))
        })?;

        Ok(token_data.claims)
    }

    /// Check if token is expired
    pub fn is_token_expired(&self, token: &str) -> bool {
        match self.extract_claims(token) {
            Ok(claims) => {
                let exp = DateTime::from_timestamp(claims.exp, 0).unwrap_or(Utc::now());
                Utc::now() > exp
            }
            Err(_) => true, // Consider invalid tokens as expired
        }
    }

    /// Get token expiration time
    pub fn get_token_expiration(&self, token: &str) -> Option<DateTime<Utc>> {
        match self.extract_claims(token) {
            Ok(claims) => DateTime::from_timestamp(claims.exp, 0),
            Err(_) => None,
        }
    }

    /// Revoke token (in practice, this would add to a blacklist)
    pub fn revoke_token(&self, token: &str) -> Result<()> {
        // In a real implementation, add the token JTI to a blacklist
        // For now, just log the revocation
        if let Ok(claims) = self.extract_claims(token) {
            tracing::info!("Token revoked: {}", claims.jti);
        }
        Ok(())
    }

    /// Additional validation logic
    fn additional_validation(&self, claims: &Claims) -> std::result::Result<(), String> {
        // Check if token is expired
        let exp = DateTime::from_timestamp(claims.exp, 0)
            .ok_or_else(|| "Invalid expiration timestamp".to_string())?;

        if Utc::now() > exp {
            return Err("Token has expired".to_string());
        }

        // Check not before time
        let nbf = DateTime::from_timestamp(claims.nbf, 0)
            .ok_or_else(|| "Invalid not before timestamp".to_string())?;

        if Utc::now() < nbf {
            return Err("Token not yet valid".to_string());
        }

        // Check if token was issued in the future (clock skew protection)
        let iat = DateTime::from_timestamp(claims.iat, 0)
            .ok_or_else(|| "Invalid issued at timestamp".to_string())?;

        if iat > Utc::now() + Duration::minutes(5) {
            return Err("Token issued in the future".to_string());
        }

        // Additional custom validation can be added here
        Ok(())
    }
}

impl Claims {
    /// Create new claims for a user
    pub fn new(user_id: Uuid, username: &str, email: &str) -> Self {
        let now = Utc::now();

        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::hours(1)).timestamp(), // Will be overridden by token generation
            nbf: now.timestamp(),
            iss: "gaussos".to_string(), // Will be overridden by JwtManager
            aud: vec!["gaussos-api".to_string()], // Will be overridden by JwtManager
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Access,
            roles: Vec::new(),
            custom: serde_json::Value::Null,
        }
    }

    /// Add role to claims
    pub fn with_role(mut self, role: &str) -> Self {
        self.roles.push(role.to_string());
        self
    }

    /// Add multiple roles to claims
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    /// Add custom claim
    pub fn with_custom_claim(mut self, key: &str, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.custom {
            map.insert(key.to_string(), value);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value);
            self.custom = serde_json::Value::Object(map);
        }
        self
    }

    /// Check if user has specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Get user ID as UUID
    pub fn user_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sub)
            .map_err(|e| GaussOSError::ValidationError(format!("Invalid user ID in claims: {}", e)))
    }

    /// Get expiration as DateTime
    pub fn expiration(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.exp, 0)
    }

    /// Get issued at as DateTime
    pub fn issued_at(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.iat, 0)
    }

    /// Check if claims are valid (not expired)
    pub fn is_valid(&self) -> bool {
        if let Some(exp) = self.expiration() {
            Utc::now() <= exp
        } else {
            false
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "your-super-secret-jwt-key-change-this-in-production".to_string(),
            algorithm: Algorithm::HS256,
            access_token_expiry_hours: 1,
            refresh_token_expiry_days: 30,
            issuer: "gaussos".to_string(),
            audience: vec!["gaussos-api".to_string()],
        }
    }
}

impl JwtConfig {
    /// Create JWT config from environment variables
    pub fn from_env() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "default-secret-change-this".to_string()),
            algorithm: Algorithm::HS256,
            access_token_expiry_hours: std::env::var("JWT_ACCESS_EXPIRY_HOURS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .unwrap_or(1),
            refresh_token_expiry_days: std::env::var("JWT_REFRESH_EXPIRY_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "gaussos".to_string()),
            audience: std::env::var("JWT_AUDIENCE")
                .unwrap_or_else(|_| "gaussos-api".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.secret.len() < 32 {
            return Err(GaussOSError::ConfigurationError(
                "JWT secret must be at least 32 characters long".to_string(),
            ));
        }

        if self.access_token_expiry_hours < 1 {
            return Err(GaussOSError::ConfigurationError(
                "Access token expiry must be at least 1 hour".to_string(),
            ));
        }

        if self.refresh_token_expiry_days < 1 {
            return Err(GaussOSError::ConfigurationError(
                "Refresh token expiry must be at least 1 day".to_string(),
            ));
        }

        if self.issuer.is_empty() {
            return Err(GaussOSError::ConfigurationError(
                "JWT issuer cannot be empty".to_string(),
            ));
        }

        if self.audience.is_empty() {
            return Err(GaussOSError::ConfigurationError(
                "JWT audience cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_config_validation() {
        let mut config = JwtConfig::default();
        assert!(config.validate().is_ok());

        // Test short secret
        config.secret = "short".to_string();
        assert!(config.validate().is_err());

        // Test invalid expiry
        config.secret = "a-very-long-secret-that-is-definitely-more-than-32-characters".to_string();
        config.access_token_expiry_hours = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_claims_creation() {
        let user_id = Uuid::new_v4();
        let claims = Claims::new(user_id, "testuser", "test@example.com")
            .with_role("user")
            .with_custom_claim("department", serde_json::json!("engineering"));

        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.email, "test@example.com");
        assert!(claims.has_role("user"));
        assert!(!claims.has_role("admin"));
        assert_eq!(claims.user_id().unwrap(), user_id);
    }

    #[tokio::test]
    async fn test_token_generation_and_validation() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = Uuid::new_v4();
        let claims = Claims::new(user_id, "testuser", "test@example.com");

        // Generate token pair
        let token_pair = jwt_manager.generate_token_pair(&claims).unwrap();
        assert!(!token_pair.access_token.is_empty());
        assert!(!token_pair.refresh_token.is_empty());

        // Validate access token
        let validation = jwt_manager.validate_token(&token_pair.access_token);
        assert!(validation.is_valid);
        assert!(validation.claims.is_some());

        let validated_claims = validation.claims.unwrap();
        assert_eq!(validated_claims.username, "testuser");
        assert_eq!(validated_claims.token_type, TokenType::Access);

        // Validate refresh token
        let refresh_validation = jwt_manager.validate_token(&token_pair.refresh_token);
        assert!(refresh_validation.is_valid);

        let refresh_claims = refresh_validation.claims.unwrap();
        assert_eq!(refresh_claims.token_type, TokenType::Refresh);
    }

    #[tokio::test]
    async fn test_token_refresh() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        let user_id = Uuid::new_v4();
        let claims = Claims::new(user_id, "testuser", "test@example.com");

        // Generate initial token pair
        let token_pair = jwt_manager.generate_token_pair(&claims).unwrap();

        // Refresh the access token
        let new_token_pair = jwt_manager
            .refresh_access_token(&token_pair.refresh_token)
            .unwrap();
        assert!(!new_token_pair.access_token.is_empty());
        assert!(!new_token_pair.refresh_token.is_empty());

        // New tokens should be different
        assert_ne!(token_pair.access_token, new_token_pair.access_token);
        assert_ne!(token_pair.refresh_token, new_token_pair.refresh_token);
    }

    #[test]
    fn test_invalid_token_validation() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();

        // Test invalid token
        let validation = jwt_manager.validate_token("invalid.token.here");
        assert!(!validation.is_valid);
        assert!(validation.error.is_some());
    }
}
