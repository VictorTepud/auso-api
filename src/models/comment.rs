use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub content: String,
    pub parent_comment_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCommentRequest {
    #[validate(length(min = 1, max = 2000, message = "El comentario debe tener entre 1 y 2000 caracteres"))]
    pub content: String,
    pub parent_comment_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub comment: Comment,
    pub author_username: String,
    pub author_display_name: String,
    pub author_profile_photo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CommentWithAuthor {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub content: String,
    pub parent_comment_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub author_username: String,
    pub author_display_name: String,
    pub author_profile_photo: Option<String>,
}
