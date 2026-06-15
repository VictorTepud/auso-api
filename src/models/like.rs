use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Like {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ToggleLikeResponse {
    pub liked: bool,
    pub likes_count: i64,
}
