-- AUSO - Migration 004
-- blocked_hashtags: when a user taps "No me interesa", the post's hashtags
-- are recorded here so the recommender can exclude posts with those tags
-- from the recommended feed.

CREATE TABLE IF NOT EXISTS blocked_hashtags (
    user_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, hashtag_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_blocked_hashtags_user_id ON blocked_hashtags(user_id);
