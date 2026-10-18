-- Shaped like `sqlite3 db .schema`: the CREATE statements come back in creation
-- order, quoted however the tool that wrote them quoted things, with
-- `sqlite_sequence` sitting among the real tables.
--
-- `crates/craber/tests/dialects.rs` asserts this parses without a warning and
-- that `sqlite_sequence` does not reach the diagram.

CREATE TABLE IF NOT EXISTS "schema_migrations" ("version" varchar NOT NULL PRIMARY KEY);
CREATE TABLE sqlite_sequence(name,seq);
CREATE TABLE IF NOT EXISTS "authors" (
  "id"         integer PRIMARY KEY AUTOINCREMENT NOT NULL,
  "name"       varchar NOT NULL,
  "created_at" datetime(6) NOT NULL
);
CREATE TABLE IF NOT EXISTS "books" (
  "id"        integer PRIMARY KEY AUTOINCREMENT NOT NULL,
  "title"     varchar NOT NULL,
  "author_id" integer NOT NULL,
  "sequel_to" integer REFERENCES "books" ("id") ON DELETE SET NULL,
  CONSTRAINT "fk_rails_53d51ce16a"
    FOREIGN KEY ("author_id")
    REFERENCES "authors" ("id")
    DEFERRABLE INITIALLY DEFERRED
);
CREATE UNIQUE INDEX "index_books_on_title" ON "books" ("title");
CREATE INDEX "index_books_on_author_id" ON "books" ("author_id");
CREATE TABLE [legacy_imports] (
  [id]  INTEGER NOT NULL,
  [ref] INTEGER REFERENCES [books]([id]),
  PRIMARY KEY([id])
) WITHOUT ROWID;
CREATE TABLE prices (id INTEGER PRIMARY KEY, book_id INT NOT NULL REFERENCES books(id), cents INT NOT NULL) STRICT;
CREATE VIEW v_books AS SELECT * FROM books;
CREATE TRIGGER trg_books_touch AFTER INSERT ON books BEGIN UPDATE authors SET name = name; END;
