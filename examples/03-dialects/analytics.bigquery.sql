-- A small analytics dataset, in BigQuery DDL. BigQuery keys are informational
-- (NOT ENFORCED), but they are what draws the edges.

CREATE TABLE `my-project.analytics.customers` (
  customer_id INT64 NOT NULL OPTIONS(description = "surrogate key"),
  email       STRING,
  signed_up   TIMESTAMP,
  PRIMARY KEY (customer_id) NOT ENFORCED
)
OPTIONS(description = "One row per customer");

CREATE TABLE `my-project.analytics.products` (
  product_id INT64 NOT NULL,
  name       STRING,
  tags       ARRAY<STRING>,
  PRIMARY KEY (product_id) NOT ENFORCED
);

CREATE TABLE `my-project.analytics.orders` (
  order_id    INT64 NOT NULL,
  customer_id INT64 REFERENCES `my-project.analytics.customers` (customer_id) NOT ENFORCED,
  ordered_at  TIMESTAMP,
  shipping    STRUCT<city STRING, postcode STRING>,
  PRIMARY KEY (order_id) NOT ENFORCED
)
PARTITION BY DATE(ordered_at)
CLUSTER BY customer_id;

CREATE TABLE `my-project.analytics.order_items` (
  order_id   INT64 NOT NULL,
  product_id INT64 NOT NULL,
  quantity   INT64 OPTIONS(description = "units, never negative"),
  PRIMARY KEY (order_id, product_id) NOT ENFORCED
);

ALTER TABLE `my-project.analytics.order_items`
  ADD CONSTRAINT order_items_order_fk
  FOREIGN KEY (order_id) REFERENCES `my-project.analytics.orders` (order_id) NOT ENFORCED;

ALTER TABLE `my-project.analytics.order_items`
  ADD CONSTRAINT order_items_product_fk
  FOREIGN KEY (product_id) REFERENCES `my-project.analytics.products` (product_id) NOT ENFORCED;
