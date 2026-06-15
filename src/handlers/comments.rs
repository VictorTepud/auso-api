use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;
use uuid::Uuid;
use validator::Validate;

use crate::config::Config;
use crate::errors::ApiError;
use crate::middleware::auth::get_user_id_from_request;
use crate::models::comment::*;

/// POST /api/v1/posts/{post_id}/comments
pub async fn create_comment(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    post_id: web::Path<String>,
    body: web::Json<CreateCommentRequest>,
) -> Result<HttpResponse, ApiError> {
    body.validate()
        .map_err(|e| ApiError::bad_request(format!("Datos inválidos: {}", e)))?;

    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let post_id = post_id.into_inner();

    // Verificar que el post existe
    let _ = sqlx::query_as::<_, crate::models::post::Post>(
        "SELECT * FROM posts WHERE id = ?"
    )
    .bind(&post_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Post no encontrado"))?;

    // Si es respuesta a otro comentario, verificar que existe
    if let Some(ref parent_id) = body.parent_comment_id {
        let _ = sqlx::query_as::<_, Comment>(
            "SELECT * FROM comments WHERE id = ? AND post_id = ?"
        )
        .bind(parent_id)
        .bind(&post_id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("Comentario padre no encontrado"))?;
    }

    let comment_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO comments (id, post_id, user_id, content, parent_comment_id) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&comment_id)
    .bind(&post_id)
    .bind(&user_id)
    .bind(&body.content)
    .bind(&body.parent_comment_id)
    .execute(pool.get_ref())
    .await?;

    let comment = sqlx::query_as::<_, CommentWithAuthor>(
        "SELECT c.*, u.username as author_username, u.display_name as author_display_name, u.profile_photo_url as author_profile_photo FROM comments c JOIN users u ON c.user_id = u.id WHERE c.id = ?"
    )
    .bind(&comment_id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(CommentResponse {
        comment: Comment {
            id: comment.id,
            post_id: comment.post_id,
            user_id: comment.user_id,
            content: comment.content,
            parent_comment_id: comment.parent_comment_id,
            created_at: comment.created_at,
            updated_at: comment.updated_at,
        },
        author_username: comment.author_username,
        author_display_name: comment.author_display_name,
        author_profile_photo: comment.author_profile_photo,
    }))
}

/// GET /api/v1/posts/{post_id}/comments
pub async fn list_comments(
    pool: web::Data<SqlitePool>,
    post_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let post_id = post_id.into_inner();

    let comments = sqlx::query_as::<_, CommentWithAuthor>(
        "SELECT c.*, u.username as author_username, u.display_name as author_display_name, u.profile_photo_url as author_profile_photo FROM comments c JOIN users u ON c.user_id = u.id WHERE c.post_id = ? ORDER BY c.created_at ASC"
    )
    .bind(&post_id)
    .fetch_all(pool.get_ref())
    .await?;

    let responses: Vec<CommentResponse> = comments
        .into_iter()
        .map(|c| CommentResponse {
            comment: Comment {
                id: c.id,
                post_id: c.post_id,
                user_id: c.user_id,
                content: c.content,
                parent_comment_id: c.parent_comment_id,
                created_at: c.created_at,
                updated_at: c.updated_at,
            },
            author_username: c.author_username,
            author_display_name: c.author_display_name,
            author_profile_photo: c.author_profile_photo,
        })
        .collect();

    Ok(HttpResponse::Ok().json(responses))
}

/// DELETE /api/v1/comments/{id}
pub async fn delete_comment(
    pool: web::Data<SqlitePool>,
    req: HttpRequest,
    config: web::Data<Config>,
    comment_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user_id = get_user_id_from_request(&req, config.get_ref())?;
    let comment_id = comment_id.into_inner();

    let comment = sqlx::query_as::<_, Comment>(
        "SELECT * FROM comments WHERE id = ?"
    )
    .bind(&comment_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("Comentario no encontrado"))?;

    if comment.user_id != user_id {
        return Err(ApiError::forbidden("No puedes eliminar este comentario"));
    }

    sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(&comment_id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Comentario eliminado"
    })))
}
