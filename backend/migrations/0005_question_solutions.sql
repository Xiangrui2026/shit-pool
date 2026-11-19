CREATE TABLE question_solutions (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    question_id INTEGER NOT NULL REFERENCES questions (id),
    title TEXT NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 120),
    body TEXT NOT NULL CHECK (char_length(body) <= 8000 AND body ~ '[^[:space:]]'),
    is_preferred BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMPTZ
);

CREATE INDEX question_solutions_active_idx
    ON question_solutions (question_id, is_preferred DESC, id)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX question_solutions_preferred_idx
    ON question_solutions (question_id)
    WHERE is_preferred AND deleted_at IS NULL;

INSERT INTO question_solutions (question_id, title, body, is_preferred)
SELECT id, '默认解答', answer, TRUE FROM questions WHERE answer ~ '[^[:space:]]';

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
       ) AS paper_source
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

ALTER TABLE questions DROP COLUMN answer;
