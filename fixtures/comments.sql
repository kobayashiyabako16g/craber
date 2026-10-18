-- A schema whose documentation is longer than its structure.
--
-- Comments are prose, and prose is what breaks a layout: a paragraph, a
-- sentence with no spaces to break at, an identifier too long for a column of
-- values. The web app's details panel puts anything longer than a few words on
-- its own full-width line for exactly these; `fixtures/tiny.sql` has short
-- comments and cannot tell whether that still works.
--
-- Use it with `just ui fixtures/comments.sql`, then click a table.

CREATE TABLE billing.invoices (
    id            bigint  PRIMARY KEY,
    number        text    NOT NULL UNIQUE,
    status        text    NOT NULL DEFAULT 'draft',
    -- A default long enough to need the full width of the panel.
    external_ref  text    DEFAULT nextval('billing.invoices_external_ref_seq'::regclass),
    total_cents   integer NOT NULL,
    issued_at     timestamptz
);

CREATE TABLE billing.invoice_lines (
    id          bigint  PRIMARY KEY,
    invoice_id  bigint  NOT NULL REFERENCES billing.invoices (id) ON DELETE CASCADE,
    description text    NOT NULL,
    amount_cents integer NOT NULL
);

CREATE INDEX invoice_lines_invoice_id_idx ON billing.invoice_lines (invoice_id);

-- A paragraph: several sentences, wrapping over four or five lines.
COMMENT ON TABLE billing.invoices IS
    'One row per issued invoice. Rows are never deleted once the invoice has been sent to the customer: a cancellation is recorded as a credit note instead, so that the sequence of invoice numbers stays gapless for the tax authority.';

-- Arrows, punctuation and a state machine, which is what column comments
-- actually tend to contain.
COMMENT ON COLUMN billing.invoices.status IS
    'draft -> issued -> paid. An invoice can also move to voided from any state before paid, which is the only transition here that is not one-way.';

-- Short. Comments always get their own line — they are prose, not a value —
-- and a one-liner should not look stranded there.
COMMENT ON COLUMN billing.invoices.number IS 'Human-facing invoice number.';

-- No spaces to break at. A layout that only breaks on spaces overflows here,
-- and CJK text is the common case for it rather than an exotic one.
COMMENT ON COLUMN billing.invoices.total_cents IS
    '請求書の通貨における最小単位での税込合計金額。丸めは行単位ではなく請求書単位で行うため、明細の合計と一致しないことがある。';

COMMENT ON COLUMN billing.invoices.external_ref IS
    'Identifier in the accounting system this invoice was exported to. Null until the nightly export has run.';

COMMENT ON COLUMN billing.invoice_lines.invoice_id IS
    'The invoice this line belongs to. Lines are deleted with their invoice, which is the one place in this schema where a cascade is intended.';
