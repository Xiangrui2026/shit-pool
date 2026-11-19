CREATE TABLE render_jobs (
    id TEXT PRIMARY KEY CHECK (id ~ '^[0-9a-f]{32}$'),
    snapshot JSONB NOT NULL,
    has_answers BOOLEAN NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'running', 'completed', 'failed')),
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);

CREATE INDEX render_jobs_pending_idx ON render_jobs (created_at, id) WHERE status = 'queued';
CREATE INDEX render_jobs_created_idx ON render_jobs (created_at);
