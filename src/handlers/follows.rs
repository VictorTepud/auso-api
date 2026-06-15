use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;

/// POST /api/v1/users/{user_id}/follow
pub async fn follow_user(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    target_user_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let target_user_id = target_user_id.into_inner();

    if user_id == target_user_id {
        return Err(ApiError::bad_request("No puedes seguirte a ti mismo"));
    }

    // Verificar que el usuario objetivo existe
    let _ = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT * FROM users WHERE id = ?"
    )
    .bind(&target_user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Usuario no encontrado"))?;

    // Verificar si ya lo sigue
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM follows WHERE follower_id = ? AND following_id = ?"
    )
    .bind(&user_id)
    .bind(&target_user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let following: bool;
    if existing > 0 {
        // Dejar de seguir
        sqlx::query("DELETE FROM follows WHERE follower_id = ? AND following_id = ?")
            .bind(&user_id)
            .bind(&target_user_id)
            .execute(pool.get_ref())
            .await?;
        following = false;
    } else {
        // Seguir
        let follow_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO follows (id, follower_id, following_id) VALUES (?, ?, ?)"
        )
        .bind(&follow_id)
        .bind(&user_id)
        .bind(&target_user_id)
        .execute(pool.get_ref())
        .await?;
        following = true;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "following": following,
    })))
}

/// GET /api/v1/users/{user_id}/followers
pub async fn get_followers(
    pool: web::Data<SqlitePool>,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = user_id.into_inner();

    let followers = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT u.* FROM users u JOIN follows f ON u.id = f.follower_id WHERE f.following_id = ? ORDER BY f.created_at DESC"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;

    let profiles: Vec<crate::models::user::UserProfile> = followers.into_iter().map(|u| u.into()).collect();

    Ok(HttpResponse::Ok().json(profiles))
}

/// GET /api/v1/users/{user_id}/following
pub async fn get_following(
    pool: web::Data<SqlitePool>,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = user_id.into_inner();

    let following = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT u.* FROM users u JOIN follows f ON u.id = f.following_id WHERE f.follower_id = ? ORDER BY f.created_at DESC"
    )
    .bind(&user_id)
    .fetch_all(pool.get_ref())
    .await?;

    let profiles: Vec<crate::models::user::UserProfile> = following.into_iter().map(|u| u.into()).collect();

    Ok(HttpResponse::Ok().json(profiles))
}
