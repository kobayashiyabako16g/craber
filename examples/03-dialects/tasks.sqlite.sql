-- SQLite-flavoured DDL: no schema namespace, INTEGER PRIMARY KEY as the rowid
-- alias, and column-level REFERENCES.
--
-- The dialect is inferred from the filename; --dialect sqlite forces it.

CREATE TABLE projects (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    archived   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE tasks (
    id         INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    parent_id  INTEGER REFERENCES tasks (id) ON DELETE CASCADE,
    title      TEXT    NOT NULL,
    done_at    TEXT
);

CREATE TABLE labels (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE task_labels (
    task_id  INTEGER NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    label_id INTEGER NOT NULL REFERENCES labels (id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, label_id)
);
