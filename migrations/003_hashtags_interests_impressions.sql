-- AUSO - Migration 003
-- Hashtags, user interests, post impressions, and categories
-- Powers the recommendation algorithm + hashtag-based discovery

-- ── Categories (global, seed list) ───────────────────────────────
-- Pre-populated so users can pick interests at registration.
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    icon TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed categories
INSERT OR IGNORE INTO categories (id, name, slug, icon) VALUES
    ('cat_tech',       'Tecnología',   'tech',       '💻'),
    ('cat_gaming',     'Gaming',       'gaming',     '🎮'),
    ('cat_music',      'Música',       'music',      '🎵'),
    ('cat_sports',     'Deportes',     'sports',     '⚽'),
    ('cat_food',       'Comida',       'food',       '🍜'),
    ('cat_travel',     'Viajes',       'travel',     '✈️'),
    ('cat_art',        'Arte',         'art',        '🎨'),
    ('cat_fashion',    'Moda',         'fashion',    '👗'),
    ('cat_movies',     'Cine',         'movies',     '🎬'),
    ('cat_books',      'Libros',       'books',      '📚'),
    ('cat_fitness',    'Fitness',      'fitness',    '💪'),
    ('cat_news',       'Noticias',     'news',       '📰'),
    ('cat_science',    'Ciencia',      'science',    '🔬'),
    ('cat_meme',       'Memes',        'meme',       '😂'),
    ('cat_photo',      'Fotografía',   'photo',      '📷'),
    ('cat_dance',      'Baile',        'dance',      '💃');

-- ── User interests (selected at registration / editable later) ──
CREATE TABLE IF NOT EXISTS user_interests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    category_id TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,  -- decays/boosts over time as the algo learns
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE,
    UNIQUE(user_id, category_id)
);
CREATE INDEX IF NOT EXISTS idx_user_interests_user_id ON user_interests(user_id);
CREATE INDEX IF NOT EXISTS idx_user_interests_category_id ON user_interests(category_id);

-- ── Hashtags ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS hashtags (
    id TEXT PRIMARY KEY,
    tag TEXT NOT NULL UNIQUE,           -- stored WITHOUT the # prefix, lowercase
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_hashtags_tag ON hashtags(tag);
CREATE INDEX IF NOT EXISTS idx_hashtags_usage_count ON hashtags(usage_count DESC);

-- ── Post ↔ Hashtag join ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS post_hashtags (
    id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL,
    hashtag_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE,
    UNIQUE(post_id, hashtag_id)
);
CREATE INDEX IF NOT EXISTS idx_post_hashtags_post_id ON post_hashtags(post_id);
CREATE INDEX IF NOT EXISTS idx_post_hashtags_hashtag_id ON post_hashtags(hashtag_id);

-- ── Post impressions (feeds the recommendation algorithm) ────────
-- Every time the user views, likes, comments, or shares a post, we record
-- an impression with a weight. The recommender sums these weights per
-- category/hashtag to compute the user's affinity vector.
CREATE TABLE IF NOT EXISTS post_impressions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    post_id TEXT NOT NULL,
    impression_type TEXT NOT NULL,     -- 'view' | 'like' | 'comment' | 'share' | 'skip'
    weight REAL NOT NULL,              -- view=1, like=5, comment=4, share=6, skip=-1
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_post_impressions_user_id ON post_impressions(user_id);
CREATE INDEX IF NOT EXISTS idx_post_impressions_post_id ON post_impressions(post_id);
CREATE INDEX IF NOT EXISTS idx_post_impressions_created_at ON post_impressions(created_at);

-- ── Category ↔ Hashtag mapping (so a #gaming tag boosts the Gaming category) ──
-- Auto-derived: when a post is tagged, we map known hashtags to categories.
-- Manual seeds for the most common tags; the recommender also falls back to
-- co-occurrence (posts in a category share hashtags) so missing mappings still work.
CREATE TABLE IF NOT EXISTS hashtag_categories (
    hashtag_id TEXT NOT NULL,
    category_id TEXT NOT NULL,
    PRIMARY KEY (hashtag_id, category_id),
    FOREIGN KEY (hashtag_id) REFERENCES hashtags(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
);
INSERT OR IGNORE INTO hashtag_categories (hashtag_id, category_id) VALUES
    ('ht_tech', 'cat_tech'), ('ht_gaming', 'cat_gaming'), ('ht_music', 'cat_music'),
    ('ht_sports', 'cat_sports'), ('ht_food', 'food'), ('ht_travel', 'cat_travel'),
    ('ht_art', 'cat_art'), ('ht_fashion', 'cat_fashion'), ('ht_movies', 'cat_movies'),
    ('ht_books', 'cat_books'), ('ht_fitness', 'cat_fitness'), ('ht_news', 'cat_news'),
    ('ht_science', 'cat_science'), ('ht_meme', 'cat_meme'), ('ht_photo', 'cat_photo'),
    ('ht_dance', 'cat_dance');
-- Note: hashtag IDs above won't exist until tags are created; that's fine, the
-- INSERT OR IGNORE just pre-seeds nothing if the hashtag rows don't exist yet.
-- Real mappings get inserted at hashtag-creation time in the handler.
