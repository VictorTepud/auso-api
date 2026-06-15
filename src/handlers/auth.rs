use actix_web::{web, HttpResponse};
use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::generate_token;
use crate::models::user::*;

/// POST /api/v1/auth/register
pub async fn register(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    // Verificar si el email ya existe
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE email = ?"
    )
    .bind(&body.email)
    .fetch_one(pool.get_ref())
    .await?;

    if existing > 0 {
        return Err(ApiError::conflict("El email ya está registrado"));
    }

    // Verificar si el username ya existe
    let existing_username = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE username = ?"
    )
    .bind(&body.username)
    .fetch_one(pool.get_ref())
    .await?;

    if existing_username > 0 {
        return Err(ApiError::conflict("El nombre de usuario ya está en uso"));
    }

    let user_id = Uuid::new_v4().to_string();
    let password_hash = hash(&body.password, DEFAULT_COST)?;

    sqlx::query(
        "INSERT INTO users (id, email, username, password_hash, display_name) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user_id)
    .bind(&body.email)
    .bind(&body.username)
    .bind(&password_hash)
    .bind(&body.username)
    .execute(pool.get_ref())
    .await?;

    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = ?"
    )
    .bind(&user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let token = generate_token(&user.id, &user.email, &user.username, config.get_ref())?;

    Ok(HttpResponse::Created().json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// POST /api/v1/auth/login
pub async fn login(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = ?"
    )
    .bind(&body.email)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::unauthorized("Email o contraseña incorrectos"))?;

    if !user.is_active {
        return Err(ApiError::forbidden("Cuenta desactivada"));
    }

    let valid = verify(&body.password, &user.password_hash)?;
    if !valid {
        return Err(ApiError::unauthorized("Email o contraseña incorrectos"));
    }

    let token = generate_token(&user.id, &user.email, &user.username, config.get_ref())?;

    Ok(HttpResponse::Ok().json(AuthResponse {
        token,
        user: user.into(),
    }))
}
