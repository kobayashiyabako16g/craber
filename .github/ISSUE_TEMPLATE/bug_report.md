---
name: Bug report
about: CrabER crashed, refused valid input, or drew the wrong diagram
title: ''
labels: bug
assignees: ''
---

<!-- Never paste a DSN, password or other credentials. Replace them with placeholders such as postgres://user:***@host/db -->

# What happened

<!-- What did you expect, and what did you get instead? Include the full error or warning output. -->

# How to reproduce

<!-- The command you ran, plus the smallest SQL that triggers the problem. -->

```console
$ craber export --file schema.sql --format svg -o er.svg
```

```sql
CREATE TABLE users (id integer PRIMARY KEY);
```

# Environment

- CrabER version (`craber --version`):
- Input (DDL file / live database, and PostgreSQL / MySQL / SQLite):
- Command (`serve` / `build` / `export`):
- Installation method (release binary / installer, cargo install, Homebrew, Nix, Docker, source):
- OS:
- Browser (only for `serve` / `build` output):

# Additional context

<!-- Screenshots, craber.toml, or anything else that helps. -->
