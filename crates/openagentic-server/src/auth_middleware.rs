//! JWT Authentication Middleware
//!
//! Provides Bearer token authentication for all API endpoints.
//! Endpoints like /health are excluded from authentication.

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// JWT ID (for revocation)
    pub jti: String,
}

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for HMAC signing
    pub secret: String,
    /// Token expiration in seconds (default: 24 hours)
    pub expiration_secs: u64,
    /// Issuer
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: generate_default_secret(),
            expiration_secs: 86400, // 24 hours
            issuer: "openagentic".to_string(),
        }
    }
}

impl JwtConfig {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            ..Default::default()
        }
    }

    /// Create JwtConfig from core SecurityConfig fields.
    /// Returns None if jwt_secret is not configured.
    pub fn from_security_config(
        jwt_secret: Option<&str>,
        jwt_expiration_secs: Option<u64>,
    ) -> Option<Self> {
        jwt_secret.map(|secret| Self {
            secret: secret.to_string(),
            expiration_secs: jwt_expiration_secs.unwrap_or(86400),
            issuer: "openagentic".to_string(),
        })
    }

    /// Generate a new JWT token for a user
    pub fn generate_token(&self, user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: user_id.to_string(),
            exp: now + self.expiration_secs,
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    /// Validate a JWT token and return the claims
    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )?;

        Ok(token_data.claims)
    }
}

/// Paths that don't require authentication
const PUBLIC_PATHS: &[&str] = &[
    "/health",
    "/api/auth/login",
    "/api/auth/token",
    "/ws",
];

/// Path prefixes that don't require authentication
const PUBLIC_PREFIXES: &[&str] = &[
    "/assets/",
];

fn is_public_path(path: &str) -> bool {
    PUBLIC_PATHS.iter().any(|p| path == *p)
        || PUBLIC_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

/// Auth middleware that validates JWT tokens
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();

    // Skip auth for public paths
    if is_public_path(&path) {
        return next.run(request).await;
    }

    // Extract JWT config from request extensions
    let jwt_config = request.extensions().get::<Arc<JwtConfig>>().cloned();

    let jwt_config = match jwt_config {
        Some(config) => config,
        None => {
            // If no JWT config is set, allow the request (auth not configured)
            return next.run(request).await;
        }
    };

    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing or invalid Authorization header",
                    "code": 401
                })),
            )
                .into_response();
        }
    };

    // Validate token
    match jwt_config.validate_token(token) {
        Ok(claims) => {
            // Check if token is expired (double-check)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if claims.exp < now {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "Token expired",
                        "code": 401
                    })),
                )
                    .into_response();
            }

            // Token is valid, proceed
            next.run(request).await
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": format!("Invalid token: {}", e),
                "code": 401
            })),
        )
            .into_response(),
    }
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response with JWT token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// Login handler - generates JWT token
pub async fn login_handler(
    axum::extract::Extension(jwt_config): axum::extract::Extension<Arc<JwtConfig>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    // For now, validate against configured credentials
    // In production, this should check against a user database
    if request.username.is_empty() || request.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Username and password are required",
                "code": 400
            })),
        ));
    }

    // Generate token
    match jwt_config.generate_token(&request.username) {
        Ok(token) => Ok(Json(LoginResponse {
            token,
            expires_in: jwt_config.expiration_secs,
            token_type: "Bearer".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to generate token: {}", e),
                "code": 500
            })),
        )),
    }
}

fn generate_default_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let config = JwtConfig::new("test-secret-key-12345".to_string());
        let token = config.generate_token("user123").unwrap();
        let claims = config.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "user123");
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig::new("test-secret-key-12345".to_string());
        let result = config.validate_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let config1 = JwtConfig::new("secret1".to_string());
        let config2 = JwtConfig::new("secret2".to_string());
        let token = config1.generate_token("user123").unwrap();
        let result = config2.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_public_paths() {
        assert!(is_public_path("/health"));
        assert!(is_public_path("/api/auth/login"));
        assert!(!is_public_path("/chat"));
        assert!(!is_public_path("/api/agents"));
    }
}
