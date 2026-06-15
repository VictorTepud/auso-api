use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        ApiError {
            status: 400,
            message: msg.into(),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        ApiError {
            status: 401,
            message: msg.into(),
        }
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        ApiError {
            status: 403,
            message: msg.into(),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        ApiError {
            status: 404,
            message: msg.into(),
        }
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        ApiError {
            status: 409,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        ApiError {
            status: 500,
            message: msg.into(),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiError {{ status: {}, message: {} }}", self.status, self.message)
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(
            actix_web::http::StatusCode::from_u16(self.status)
                .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
        )
        .json(self)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::not_found("Recurso no encontrado"),
            sqlx::Error::Database(ref db_err) => {
                if db_err.code().map(|c| c == "2067" || c == "1555").unwrap_or(false) {
                    ApiError::conflict("El recurso ya existe")
                } else {
                    ApiError::internal(format!("Error de base de datos: {}", db_err.message()))
                }
            }
            _ => ApiError::internal(format!("Error de base de datos: {}", err)),
        }
    }
}

impl From<bcrypt::BcryptError> for ApiError {
    fn from(_: bcrypt::BcryptError) -> Self {
        ApiError::internal("Error al procesar la contraseña")
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        ApiError::unauthorized("Token inválido")
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
