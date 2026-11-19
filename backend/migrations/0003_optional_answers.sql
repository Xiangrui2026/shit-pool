ALTER TABLE questions DROP CONSTRAINT questions_answer_check;
ALTER TABLE questions ALTER COLUMN answer SET DEFAULT '';
ALTER TABLE questions ADD CONSTRAINT questions_answer_check CHECK (char_length(answer) <= 8000);
