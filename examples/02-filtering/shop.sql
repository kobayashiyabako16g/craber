-- A storefront schema with the noise a real dump carries: migration bookkeeping,
-- an audit trail, and a session table nobody wants on the diagram.
--
-- Used to demonstrate --include / --exclude / --depth and craber.toml.

CREATE TABLE public.customers (
    id            bigint      PRIMARY KEY,
    email         text        NOT NULL UNIQUE,
    full_name     text        NOT NULL,
    signed_up_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.addresses (
    id          bigint PRIMARY KEY,
    customer_id bigint NOT NULL REFERENCES public.customers (id) ON DELETE CASCADE,
    line1       text   NOT NULL,
    city        text   NOT NULL,
    country     char(2) NOT NULL
);

CREATE TABLE public.products (
    id          bigint         PRIMARY KEY,
    sku         text           NOT NULL UNIQUE,
    name        text           NOT NULL,
    price_cents integer        NOT NULL,
    weight_g    integer
);

CREATE TABLE public.categories (
    id        bigint PRIMARY KEY,
    parent_id bigint REFERENCES public.categories (id) ON DELETE SET NULL,
    name      text   NOT NULL
);

CREATE TABLE public.product_categories (
    product_id  bigint NOT NULL REFERENCES public.products (id)   ON DELETE CASCADE,
    category_id bigint NOT NULL REFERENCES public.categories (id) ON DELETE CASCADE,
    PRIMARY KEY (product_id, category_id)
);

CREATE TABLE public.orders (
    id             bigint         PRIMARY KEY,
    customer_id    bigint         NOT NULL REFERENCES public.customers (id),
    shipping_id    bigint         REFERENCES public.addresses (id),
    status         text           NOT NULL DEFAULT 'pending',
    total_cents    integer        NOT NULL DEFAULT 0,
    placed_at      timestamptz    NOT NULL DEFAULT now()
);

CREATE TABLE public.order_items (
    order_id   bigint  NOT NULL REFERENCES public.orders (id)   ON DELETE CASCADE,
    product_id bigint  NOT NULL REFERENCES public.products (id),
    quantity   integer NOT NULL DEFAULT 1,
    unit_cents integer NOT NULL,
    PRIMARY KEY (order_id, product_id)
);

CREATE TABLE public.payments (
    id         bigint      PRIMARY KEY,
    order_id   bigint      NOT NULL UNIQUE REFERENCES public.orders (id),
    provider   text        NOT NULL,
    amount_cents integer   NOT NULL,
    captured_at timestamptz
);

CREATE TABLE public.shipments (
    id          bigint      PRIMARY KEY,
    order_id    bigint      NOT NULL REFERENCES public.orders (id) ON DELETE CASCADE,
    address_id  bigint      NOT NULL REFERENCES public.addresses (id),
    carrier     text        NOT NULL,
    shipped_at  timestamptz
);

-- What the tables are for, where the name alone does not say. The viewer marks
-- a commented table or column on the canvas and shows the text in its details.

COMMENT ON TABLE public.customers IS 'Anyone who has signed up, whether or not they have ordered.';
COMMENT ON COLUMN public.customers.email IS 'Used to sign in; stored lower-cased.';

COMMENT ON TABLE public.orders IS 'One checkout. Its lines, payment and shipments hang off it.';
COMMENT ON COLUMN public.orders.shipping_id IS 'NULL for orders with nothing to ship, such as gift cards.';
COMMENT ON COLUMN public.orders.status IS 'pending, paid, shipped or cancelled.';
COMMENT ON COLUMN public.orders.total_cents IS 'Sum of the lines at checkout, in the store currency.';

COMMENT ON COLUMN public.order_items.unit_cents IS 'Price when ordered; later price changes do not touch it.';

COMMENT ON TABLE public.payments IS 'At most one per order: a failed attempt is retried in place.';
COMMENT ON COLUMN public.payments.captured_at IS 'NULL until the provider confirms the charge.';

COMMENT ON TABLE public.shipments IS 'An order can ship in several parcels.';

COMMENT ON TABLE public.categories IS 'A tree: parent_id is NULL at the top level.';

-- Infrastructure tables. Every schema has them; no diagram wants them.

CREATE TABLE public.schema_migrations (
    version bigint      PRIMARY KEY,
    applied_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.audit_log (
    id         bigint      PRIMARY KEY,
    actor_id   bigint      REFERENCES public.customers (id),
    table_name text        NOT NULL,
    row_id     bigint      NOT NULL,
    action     text        NOT NULL,
    logged_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.audit_log_archive (
    id        bigint      PRIMARY KEY,
    logged_at timestamptz NOT NULL,
    payload   jsonb       NOT NULL
);

CREATE TABLE public.sessions (
    token       text        PRIMARY KEY,
    customer_id bigint      REFERENCES public.customers (id) ON DELETE CASCADE,
    expires_at  timestamptz NOT NULL
);
