use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Channel {
    pub id: String,
    pub community_id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateChannelRequest {
    #[validate(length(min = 2, max = 100, message = "El nombre debe tener entre 2 y 100 caracteres"))]
    pub name: String,
    #[validate(length(max = 1000, message = "La descripción no puede exceder 1000 caracteres"))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateChannelRequest {
    #[validate(length(min = 2, max = 100))]
    pub name: Option<String>,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub channel: Channel,
    pub posts_count: i64,
}
