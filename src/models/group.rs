use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cover_photo_url: Option<String>,
    pub creator_id: String,
    pub is_private: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMember {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateGroupRequest {
    #[validate(length(min = 3, max = 100, message = "El nombre debe tener entre 3 y 100 caracteres"))]
    pub name: String,
    #[validate(length(max = 2000, message = "La descripción no puede exceder 2000 caracteres"))]
    pub description: Option<String>,
    pub is_private: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateGroupRequest {
    #[validate(length(min = 3, max = 100))]
    pub name: Option<String>,
    #[validate(length(max = 2000))]
    pub description: Option<String>,
    pub is_private: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct GroupResponse {
    pub group: Group,
    pub members_count: i64,
    pub is_member: bool,
    pub user_role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct GroupWithCount {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cover_photo_url: Option<String>,
    pub creator_id: String,
    pub is_private: bool,
    pub created_at: String,
    pub updated_at: String,
    pub members_count: i64,
}
