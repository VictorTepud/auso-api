use actix_web::{dev::ServiceRequest, Error};
use actix_web::web;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // user id
    pub email: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn generate_token(user_id: &str, email: &str, username: &str, config: &Config) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now();
    let expiration = now + chrono::Duration::hours(config.jwt_expiration_hours);

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        username: username.to_string(),
        exp: expiration.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn extract_user_id_from_request(req: &ServiceRequest) -> Result<String, Error> {
    let config = req
        .app_data::<web::Data<Config>>()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Configuración no disponible"))?;

    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Token no proporcionado"))?;

    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        auth_header
    };

    let claims = validate_token(token, &config.jwt_secret)
        .map_err(|_| actix_web::error::ErrorUnauthorized("Token inválido o expirado"))?;

    Ok(claims.sub)
}

/// Extrae el user_id del request en los handlers
pub fn get_user_id_from_request(req: &actix_web::HttpRequest, config: &Config) -> Result<String, crate::errors::ApiError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| crate::errors::ApiError::unauthorized("Token no proporcionado"))?;

    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        auth_header
    };

    let claims = validate_token(token, &config.jwt_secret)?;
    Ok(claims.sub)
}
