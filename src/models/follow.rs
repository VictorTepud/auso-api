use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct Follow {
    pub id: String,
    pub follower_id: String,
    pub following_id: String,
    pub created_at: String,
}
