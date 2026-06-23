use actix_web::web;

use crate::handlers::*;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // ========== AUTH ==========
            .route("/auth/register", web::post().to(auth::register))
            .route("/auth/login", web::post().to(auth::login))

            // ========== USERS ==========
            .route("/users/me", web::get().to(users::get_me))
            .route("/users/me", web::put().to(users::update_profile))
            .route("/users/me/profile-photo", web::post().to(users::upload_profile_photo))
            .route("/users/me/cover-photo", web::post().to(users::upload_cover_photo))
            .route("/users/search", web::get().to(users::search_users))
            .route("/users/{username}", web::get().to(users::get_user_profile))

            // ========== FOLLOWS ==========
            .route("/users/{user_id}/follow", web::post().to(follows::follow_user))
            .route("/users/{user_id}/followers", web::get().to(follows::get_followers))
            .route("/users/{user_id}/following", web::get().to(follows::get_following))

            // ========== POSTS ==========
            .route("/posts/text", web::post().to(posts::create_text_post))
            .route("/posts/image", web::post().to(posts::create_image_post))
            .route("/posts/image-pack", web::post().to(posts::create_image_pack_post))
            .route("/posts/video", web::post().to(posts::create_video_post))
            .route("/posts/poll", web::post().to(polls::create_poll_post))
            .route("/posts/feed", web::get().to(posts::get_feed))
            .route("/posts/recommended", web::get().to(recommend::get_recommended_feed))
            .route("/posts/{id}", web::get().to(posts::get_post))
            .route("/posts/{id}", web::delete().to(posts::delete_post))
            .route("/posts/{id}/like", web::post().to(posts::toggle_like))
            .route("/posts/{id}/images", web::post().to(posts::add_images_to_post))
            .route("/posts/{id}/impression", web::post().to(discovery::record_impression))

            // ========== DISCOVERY (hashtags, categories, interests) ==========
            .route("/categories", web::get().to(discovery::list_categories))
            .route("/users/me/interests", web::post().to(discovery::set_user_interests))
            .route("/users/me/interests", web::get().to(discovery::get_my_interests))
            .route("/hashtags/search", web::get().to(discovery::search_hashtags))
            .route("/hashtags/trending", web::get().to(discovery::trending_hashtags))
            .route("/hashtags/{tag}/posts", web::get().to(discovery::get_posts_by_hashtag))

            // ========== COMMENTS ==========
            .route("/posts/{post_id}/comments", web::post().to(comments::create_comment))
            .route("/posts/{post_id}/comments", web::get().to(comments::list_comments))
            .route("/comments/{id}", web::delete().to(comments::delete_comment))

            // ========== POLLS ==========
            .route("/polls/{id}/vote", web::post().to(polls::vote_poll))
            .route("/polls/{id}/results", web::get().to(polls::get_poll_results))

            // ========== COMMUNITIES ==========
            .route("/communities", web::post().to(communities::create_community))
            .route("/communities", web::get().to(communities::list_communities))
            .route("/communities/{id}", web::get().to(communities::get_community))
            .route("/communities/{id}", web::put().to(communities::update_community))
            .route("/communities/{id}/join", web::post().to(communities::join_community))
            .route("/communities/{id}/leave", web::post().to(communities::leave_community))
            .route("/communities/{id}/members", web::get().to(communities::get_community_members))
            .route("/communities/{id}/cover-photo", web::post().to(communities::upload_community_cover))

            // ========== CHANNELS ==========
            .route("/communities/{community_id}/channels", web::post().to(channels::create_channel))
            .route("/communities/{community_id}/channels", web::get().to(channels::list_channels))
            .route("/channels/{id}", web::get().to(channels::get_channel))
            .route("/channels/{id}", web::put().to(channels::update_channel))
            .route("/channels/{id}", web::delete().to(channels::delete_channel))

            // ========== GROUPS ==========
            .route("/groups", web::post().to(groups::create_group))
            .route("/groups", web::get().to(groups::list_groups))
            .route("/groups/{id}", web::get().to(groups::get_group))
            .route("/groups/{id}", web::put().to(groups::update_group))
            .route("/groups/{id}/join", web::post().to(groups::join_group))
            .route("/groups/{id}/leave", web::post().to(groups::leave_group))
            .route("/groups/{id}/members", web::get().to(groups::get_group_members))
            .route("/groups/{id}/cover-photo", web::post().to(groups::upload_group_cover))

            // ========== VIDEO STREAMING ==========
            .route("/stream/{video_id}/master.m3u8", web::get().to(streaming::stream_hls_master))
            .route("/stream/{video_id}/thumbnail.jpg", web::get().to(streaming::stream_thumbnail))
            .route("/stream/{video_id}/{segment}", web::get().to(streaming::stream_hls_segment))
    )
    // ========== STATIC FILES ==========
    .route("/uploads/{path:.*}", web::get().to(media::serve_upload));
}
