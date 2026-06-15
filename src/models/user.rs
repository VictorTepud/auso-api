use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub bio: String,
    pub profile_photo_url: Option<String>,
    pub cover_photo_url: Option<String>,
    pub phone: String,
    pub location: String,
    pub website: String,
    pub birth_date: Option<String>,
    pub gender: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub profile_photo_url: Option<String>,
    pub cover_photo_url: Option<String>,
    pub phone: String,
    pub location: String,
    pub website: String,
    pub birth_date: Option<String>,
    pub gender: String,
    pub created_at: String,
}

impl From<User> for UserProfile {
    fn from(u: User) -> Self {
        UserProfile {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            bio: u.bio,
            profile_photo_url: u.profile_photo_url,
            cover_photo_url: u.cover_photo_url,
            phone: u.phone,
            location: u.location,
            website: u.website,
            birth_date: u.birth_date,
            gender: u.gender,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Email inválido"))]
    pub email: String,
    #[validate(length(min = 6, message = "La contraseña debe tener al menos 6 caracteres"))]
    pub password: String,
    #[validate(length(min = 3, max = 30, message = "El usuario debe tener entre 3 y 30 caracteres"))]
    pub username: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Email inválido"))]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(max = 50))]
    pub display_name: Option<String>,
    #[validate(length(max = 500))]
    pub bio: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub birth_date: Option<String>,
    pub gender: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserStats {
    pub posts_count: i64,
    pub followers_count: i64,
    pub following_count: i64,
    pub communities_count: i64,
    pub groups_count: i64,
}
