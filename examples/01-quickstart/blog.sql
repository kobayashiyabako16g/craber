-- A small blogging schema: the shape most people already have in their head,
-- so the diagram can be checked against expectations at a glance.
--
-- Dialect: PostgreSQL (the default when --dialect is omitted).

CREATE TABLE public.authors (
    id           bigint      PRIMARY KEY,
    handle       text        NOT NULL UNIQUE,
    display_name text        NOT NULL,
    bio          text,
    joined_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.posts (
    id           bigint      PRIMARY KEY,
    author_id    bigint      NOT NULL REFERENCES public.authors (id) ON DELETE CASCADE,
    slug         text        NOT NULL UNIQUE,
    title        text        NOT NULL,
    body         text        NOT NULL,
    published_at timestamptz,
    view_count   integer     NOT NULL DEFAULT 0
);

CREATE TABLE public.tags (
    id   bigint PRIMARY KEY,
    name text   NOT NULL UNIQUE
);

-- A join table: two foreign keys and a composite primary key. CrabER reads the
-- key columns, so both edges land on the right rows of the node.
CREATE TABLE public.post_tags (
    post_id bigint NOT NULL REFERENCES public.posts (id) ON DELETE CASCADE,
    tag_id  bigint NOT NULL REFERENCES public.tags (id)  ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- A self-referencing table: comments reply to other comments.
CREATE TABLE public.comments (
    id        bigint      PRIMARY KEY,
    post_id   bigint      NOT NULL,
    author_id bigint      REFERENCES public.authors (id) ON DELETE SET NULL,
    parent_id bigint,
    body      text        NOT NULL,
    posted_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT comments_post_id_fkey
        FOREIGN KEY (post_id) REFERENCES public.posts (id) ON DELETE CASCADE,
    CONSTRAINT comments_parent_id_fkey
        FOREIGN KEY (parent_id) REFERENCES public.comments (id) ON DELETE CASCADE
);

-- CrabER does not read CREATE INDEX yet (M1). It warns on stderr and draws the
-- five tables above anyway — one unreadable statement never costs the diagram.
CREATE INDEX posts_published_at_idx ON public.posts (published_at DESC);
