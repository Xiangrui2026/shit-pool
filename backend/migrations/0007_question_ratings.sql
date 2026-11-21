ALTER TABLE questions ADD COLUMN score BIGINT NOT NULL DEFAULT 0;

CREATE INDEX questions_ranking_idx ON questions (score, id) WHERE deleted_at IS NULL;

CREATE INDEX questions_active_tags_idx ON questions
    USING GIN (string_to_array(tags, ','))
    WHERE deleted_at IS NULL;

CREATE OR REPLACE VIEW question_papers AS
SELECT q.id, q.category_id, q.source, q.stem,
       COALESCE(preferred.body, ''::text) AS answer, q.notes, q.tags,
       COALESCE(
           q.original_number,
           NULLIF(translate(legacy.parts[2], '０１２３４５６７８９', '0123456789')::integer, 0)
       ) AS original_number,
       COALESCE(
           NULLIF(regexp_replace(legacy.parts[1], '[[:space:]　·:：—–-]+$', ''), ''),
           q.source
       ) AS paper_source,
       q.score
FROM questions AS q
LEFT JOIN LATERAL regexp_match(
    q.source,
    '^(.+?)第[[:space:]　]*([0-9０-９]{1,9})[[:space:]　]*题(?:[[:space:]　·:：—–（(、-].*|$)'
) AS legacy(parts) ON TRUE
LEFT JOIN LATERAL (
    SELECT body FROM question_solutions
    WHERE question_id = q.id AND deleted_at IS NULL
    ORDER BY is_preferred DESC, id LIMIT 1
) AS preferred ON TRUE
WHERE q.deleted_at IS NULL;
