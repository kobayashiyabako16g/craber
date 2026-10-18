-- MySQL-flavoured DDL: backticks, AUTO_INCREMENT, ENGINE/CHARSET table options,
-- and no schema qualifier on table names.
--
-- The dialect is inferred from the filename ("mysql" appears in it); pass
-- --dialect mysql explicitly when your dump is named something else.

CREATE TABLE `warehouses` (
  `id`      int unsigned NOT NULL AUTO_INCREMENT,
  `code`    varchar(16)  NOT NULL,
  `city`    varchar(64)  NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `warehouses_code_uniq` (`code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `items` (
  `id`       int unsigned NOT NULL AUTO_INCREMENT,
  `sku`      varchar(32)  NOT NULL,
  `name`     varchar(128) NOT NULL,
  `unit`     varchar(8)   NOT NULL DEFAULT 'ea',
  PRIMARY KEY (`id`),
  UNIQUE KEY `items_sku_uniq` (`sku`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `stock_levels` (
  `warehouse_id` int unsigned NOT NULL,
  `item_id`      int unsigned NOT NULL,
  `quantity`     int          NOT NULL DEFAULT 0,
  `counted_at`   datetime     NULL,
  PRIMARY KEY (`warehouse_id`, `item_id`),
  CONSTRAINT `stock_levels_warehouse_fk`
    FOREIGN KEY (`warehouse_id`) REFERENCES `warehouses` (`id`) ON DELETE CASCADE,
  CONSTRAINT `stock_levels_item_fk`
    FOREIGN KEY (`item_id`) REFERENCES `items` (`id`) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `stock_moves` (
  `id`           bigint unsigned NOT NULL AUTO_INCREMENT,
  `item_id`      int unsigned    NOT NULL,
  `from_id`      int unsigned    NULL,
  `to_id`        int unsigned    NULL,
  `quantity`     int             NOT NULL,
  `moved_at`     datetime        NOT NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `stock_moves_item_fk` FOREIGN KEY (`item_id`) REFERENCES `items` (`id`),
  CONSTRAINT `stock_moves_from_fk` FOREIGN KEY (`from_id`) REFERENCES `warehouses` (`id`),
  CONSTRAINT `stock_moves_to_fk`   FOREIGN KEY (`to_id`)   REFERENCES `warehouses` (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
