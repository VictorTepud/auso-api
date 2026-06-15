use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::user::*;
use crate::services::media;

/// GET /api/v1/users/me
pub async fn get_me(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(pool.get_ref())
        .await?;

    let stats = get_user_stats(pool.get_ref(), &user_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user:": UserProfile::from(user),
        "stats": stats,
    })))
}

/// GET /api/v1/users/{username}
pub async fn get_user_profile(
    pool: web::Data<SqlitePool>,
    username: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username.into_inner())
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("Usuario no encontrado"))?;

    let stats = get_user_stats(pool.get_ref(), &user.id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user": UserProfile::from(user),
        "stats": stats,
    })))
}

/// PUT /api/v1/users/me
pub async fn update_profile(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(pool.get_ref())
        .await?;

    let display_name = body.display_name.as_deref().unwrap_or(&user.display_name);
    let bio = body.bio.as_deref().unwrap_or(&user.bio);
    let phone = body.phone.as_deref().unwrap_or(&user.phone);
    let location = body.location.as_deref().unwrap_or(&user.location);
    let website = body.website.as_deref().unwrap_or(&user.website);
    let birth_date = body.birth_date.as_deref().or(user.birth_date.as_deref());
    let gender = body.gender.as_deref().unwrap_or(&user.gender);

    sqlx::query(
        "UPDATE users SET display_name=?, bio=?, phone=?, location=?, website=?, birth_date=?, gender=?, updated_at=datetime('now') WHERE id=?"
    )
    .bind(display_name)
    .bind(bio)
    .bind(phone)
    .bind(location)
    .bind(website)
    .bind(birth_date)
    .bind(gender)
    .bind(&user_id)
    .execute(pool.get_ref())
    .await?;

    let updated = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(UserProfile::from(updated)))
}

/// POST /api/v1/users/me/profile-photo
pub async fn upload_profile_photo(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;

    let uploaded = media::save_profile_image(payload, config.get_ref(), "profile").await?;

    sqlx::query("UPDATE users SET profile_photo_url=?, updated_at=datetime('now') WHERE id=?")
        .bind(&uploaded.url)
        .bind(&user_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "url": uploaded.url,
    })))
}

/// POST /api/v1/users/me/cover-photo
pub async fn upload_cover_photo(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    payload: Multipart,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;

    let uploaded = media::save_profile_image(payload, config.get_ref(), "cover").await?;

    sqlx::query("UPDATE users SET cover_photo_url=?, updated_at=datetime('now') WHERE id=?")
        .bind(&uploaded.url)
        .bind(&user_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "url": uploaded.url,
    })))
}

/// GET /api/v1/users/search?q=term
pub async fn search_users(
    pool: web::Data<SqlitePool>,
    query: web::Query<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    let search_term = query.get("q")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if search_term.is_empty() {
        return Ok(HttpResponse::Ok().json(Vec::<UserProfile>::new()));
    }

    let pattern = format!("%{}%", search_term);

    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE username LIKE ? OR display_name LIKE ? LIMIT 20"
    )
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool.get_ref())
    .await?;

    let profiles: Vec<UserProfile> = users.into_iter().map(|u| u.into()).collect();

    Ok(HttpResponse::Ok().json(profiles))
}

async fn get_user_stats(pool: &SqlitePool, user_id: &str) -> Result<UserStats, ApiError> {
    let posts_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM posts WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let followers_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM follows WHERE following_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let following_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM follows WHERE follower_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let communities_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM community_members WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let groups_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM group_members WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(UserStats {
        posts_count,
        followers_count,
        following_count,
        communities_count,
        groups_count,
    })
}
