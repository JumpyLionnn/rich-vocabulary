CREATE TABLE IF NOT EXISTS "words"(
    "uid" INTEGER PRIMARY KEY NOT NULL,
    "word" VARCHAR NOT NULL,
    "last_quizzed" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "score" INTEGER NOT NULL
);