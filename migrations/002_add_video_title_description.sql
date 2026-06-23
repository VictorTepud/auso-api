-- AUSO - Migration 002
-- Add title and description columns to post_videos table
-- Allows video posts to have a separate title and description
-- (in addition to the post.content which is used as a general caption)

ALTER TABLE post_videos ADD COLUMN title TEXT NOT NULL DEFAULT '';
ALTER TABLE post_videos ADD COLUMN description TEXT NOT NULL DEFAULT '';
