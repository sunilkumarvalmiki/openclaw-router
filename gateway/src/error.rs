use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;

/// Structured JSON error body returned to clients.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: &'static str,
    pub message: String,
}

/// Application-wide error enum.
///
/// Each variant maps to an HTTP status code and produces a JSON error body
/// when rendered through Actix-Web's `ResponseError` trait.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl AppError {
    /// Returns the error code string used in the JSON response body.
    fn error_code(&self) -> &'static str {
        match self {
            AppError::Internal(_) => "INTERNAL_ERROR",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::RateLimit(_) => "RATE_LIMIT_EXCEEDED",
            AppError::ProviderError(_) => "PROVIDER_ERROR",
            AppError::CacheError(_) => "CACHE_ERROR",
            AppError::DatabaseError(_) => "DATABASE_ERROR",
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        match self {
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::RateLimit(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::ProviderError(_) => StatusCode::BAD_GATEWAY,
            AppError::CacheError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.error_code(),
                message: self.to_string(),
            },
        };

        HttpResponse::build(self.status_code()).json(body)
    }
}

// ---------------------------------------------------------------------------
// From conversions for external error types
// ---------------------------------------------------------------------------

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "Database error");
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".into()),
            _ => AppError::DatabaseError(err.to_string()),
        }
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        tracing::error!(error = %err, "Redis error");
        AppError::CacheError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        tracing::error!(error = %err, "HTTP client error");
        if err.is_timeout() {
            AppError::ProviderError(format!("Provider request timed out: {err}"))
        } else if err.is_connect() {
            AppError::ProviderError(format!("Failed to connect to provider: {err}"))
        } else {
            AppError::ProviderError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AppError::Internal("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::BadRequest("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Unauthorized("test".into()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::NotFound("test".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::RateLimit("test".into()).status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            AppError::ProviderError("test".into()).status_code(),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            AppError::CacheError("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::DatabaseError("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(AppError::Internal("x".into()).error_code(), "INTERNAL_ERROR");
        assert_eq!(AppError::BadRequest("x".into()).error_code(), "BAD_REQUEST");
        assert_eq!(
            AppError::Unauthorized("x".into()).error_code(),
            "UNAUTHORIZED"
        );
        assert_eq!(AppError::NotFound("x".into()).error_code(), "NOT_FOUND");
        assert_eq!(
            AppError::RateLimit("x".into()).error_code(),
            "RATE_LIMIT_EXCEEDED"
        );
        assert_eq!(
            AppError::ProviderError("x".into()).error_code(),
            "PROVIDER_ERROR"
        );
        assert_eq!(AppError::CacheError("x".into()).error_code(), "CACHE_ERROR");
        assert_eq!(
            AppError::DatabaseError("x".into()).error_code(),
            "DATABASE_ERROR"
        );
    }

    #[test]
    fn test_display() {
        let err = AppError::BadRequest("missing field".into());
        assert_eq!(err.to_string(), "Bad request: missing field");
    }
}
