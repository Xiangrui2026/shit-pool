CREATE TABLE categories (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT NOT NULL UNIQUE CHECK (char_length(btrim(name)) BETWEEN 1 AND 120)
);

CREATE TABLE questions (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    category_id INTEGER NOT NULL REFERENCES categories (id),
    source TEXT NOT NULL CHECK (char_length(btrim(source)) BETWEEN 1 AND 300),
    stem TEXT NOT NULL CHECK (char_length(btrim(stem)) BETWEEN 1 AND 6000),
    answer TEXT NOT NULL CHECK (char_length(btrim(answer)) BETWEEN 1 AND 8000),
    notes TEXT NOT NULL DEFAULT '' CHECK (char_length(notes) <= 2000),
    tags TEXT NOT NULL DEFAULT '' CHECK (char_length(tags) <= 500)
);

CREATE INDEX questions_category_id_idx ON questions (category_id, id);
