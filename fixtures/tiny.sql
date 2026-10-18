-- A small schema used to smoke-test the CrabER pipeline end to end, and to
-- show every relationship end the notation can draw.
--
-- It is deliberately shaped like a pg_dump: every table is created first and
-- the foreign keys, indexes and comments arrive afterwards, referring back to
-- tables that already exist. That is the shape most real input has, and it is
-- what the parser's two-pass translation exists for.
--
-- The four foreign keys below cover every combination CrabER can derive from
-- DDL. A relationship end is decided by two facts about the foreign key:
--
--   is it over a unique tuple?  -> one row there, rather than many
--   are its columns nullable?   -> the row need not have a parent
--
-- | foreign key                 | unique | nullable | referenced end | child end   |
-- |-----------------------------|--------|----------|----------------|-------------|
-- | orders.user_id              | no     | no       | one and only one | zero or many |
-- | orders.coupon_id            | no     | yes      | zero or one    | zero or many |
-- | user_profiles.user_id       | yes    | no       | one and only one | zero or one |
-- | order_receipts.order_id     | yes    | yes      | zero or one    | zero or one |
--
-- `one or many` and a bare `one` are not in the table because no dialect states
-- them: nothing in DDL says a row must have at least one child.
--
-- It also includes one statement CrabER does not apply, so the partial-failure
-- path (warn and keep going) stays exercised by the tests.

CREATE TYPE public.order_status AS ENUM ('pending', 'paid', 'shipped');

CREATE TABLE public.users (
    id         bigint       PRIMARY KEY,
    email      text         NOT NULL UNIQUE,
    created_at timestamptz  NOT NULL DEFAULT now()
);

-- Unique and NOT NULL: one and only one user, and at most one profile for it.
CREATE TABLE public.user_profiles (
    user_id      bigint PRIMARY KEY REFERENCES public.users (id) ON DELETE CASCADE,
    display_name text   NOT NULL
);

CREATE TABLE public.coupons (
    id   bigint PRIMARY KEY,
    code text   NOT NULL UNIQUE
);

CREATE TABLE public.orders (
    id        bigint  PRIMARY KEY,
    -- NOT NULL: every order belongs to exactly one user.
    user_id   bigint  NOT NULL REFERENCES public.users (id) ON DELETE CASCADE,
    -- Nullable: an order may have been placed without a coupon.
    coupon_id bigint  REFERENCES public.coupons (id) ON DELETE SET NULL,
    status    public.order_status NOT NULL DEFAULT 'pending',
    total     numeric(12, 2) NOT NULL DEFAULT 0
);

-- Unique but nullable: a receipt is issued for at most one order, and is
-- written before the order it belongs to is known.
CREATE TABLE public.order_receipts (
    id        bigint      PRIMARY KEY,
    order_id  bigint      UNIQUE REFERENCES public.orders (id),
    issued_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.order_items (
    order_id   bigint  NOT NULL,
    product_id bigint  NOT NULL,
    quantity   integer NOT NULL DEFAULT 1,
    PRIMARY KEY (order_id, product_id)
);

-- The foreign key arrives after the table, as a dump writes it. Its column is
-- part of a composite primary key, so the tuple is not unique on its own.
ALTER TABLE ONLY public.order_items
    ADD CONSTRAINT order_items_order_id_fkey
    FOREIGN KEY (order_id) REFERENCES public.orders (id) ON DELETE CASCADE;

CREATE INDEX orders_user_id_idx ON public.orders (user_id);

COMMENT ON TABLE public.users IS 'People who can sign in.';

COMMENT ON COLUMN public.orders.total IS 'Gross total, in the order currency.';

-- Not applied: CrabER should warn and still draw the tables above.
ALTER TABLE ONLY public.orders ALTER COLUMN quantity SET NOT NULL;
