pub mod schema;
pub mod models;
pub mod db;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

impl From<db::DbError> for StatusCode {
    fn from(e: db::DbError) -> Self {
        match e {
            db::DbError::Diesel(diesel::result::Error::NotFound) => StatusCode::NOT_FOUND,
            db::DbError::Diesel(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation, _,
            )) => StatusCode::CONFLICT,
            db::DbError::Forbidden(_) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub struct AppError(pub db::DbError);

impl From<db::DbError> for AppError {
    fn from(e: db::DbError) -> Self { AppError(e) }
}

impl From<StatusCode> for AppError {
    fn from(s: StatusCode) -> Self {
        AppError(db::DbError::Forbidden(s.canonical_reason().unwrap_or("Error").to_string()))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self.0 {
            db::DbError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg).into_response(),
            db::DbError::Diesel(diesel::result::Error::NotFound) => StatusCode::NOT_FOUND.into_response(),
            db::DbError::Diesel(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation, _,
            )) => StatusCode::CONFLICT.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
