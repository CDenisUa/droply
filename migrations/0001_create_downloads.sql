CREATE TABLE downloads (
    id UUID PRIMARY KEY,
    source_url TEXT NOT NULL,
    file_name TEXT NOT NULL,
    media_type TEXT,
    status TEXT NOT NULL,
    bytes_downloaded BIGINT NOT NULL DEFAULT 0,
    total_bytes BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error TEXT
);

-- Status is a plain TEXT column, not a Postgres ENUM: adding a new
-- DownloadStatus variant should never require a migration, only a code
-- change (droply_domain::DownloadStatus::{as_str,parse} is the single
-- source of truth for valid values).
CREATE INDEX idx_downloads_status ON downloads (status);

-- Backs the "History" / recent-downloads view (doc §2).
CREATE INDEX idx_downloads_created_at ON downloads (created_at DESC);
