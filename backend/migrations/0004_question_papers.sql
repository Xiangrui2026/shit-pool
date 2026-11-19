ALTER TABLE questions ADD COLUMN original_number INTEGER CHECK (original_number > 0);

CREATE VIEW question_papers AS
SELECT q.id, q.category_id, q.source, q.stem, q.answer, q.notes, q.tags,
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
WHERE q.deleted_at IS NULL;
