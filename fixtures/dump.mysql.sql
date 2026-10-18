-- Shaped like `mysqldump --no-data`, not like DDL anyone would write by hand:
-- the /*!…*/ version comments, the DROP/LOCK scaffolding and the `KEY` line per
-- index are all things a real dump has and `examples/03-dialects` does not.
--
-- `crates/craber/tests/dialects.rs` asserts this parses without a single
-- warning, which is the point: every line below used to be either dropped or
-- reported as unrepresentable.

/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40103 SET TIME_ZONE='+00:00' */;

DROP TABLE IF EXISTS `users`;
/*!40101 SET @saved_cs_client = @@character_set_client */;
CREATE TABLE `users` (
  `id`           bigint unsigned NOT NULL AUTO_INCREMENT,
  `email`        varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
  `role`         enum('admin','member','guest') NOT NULL DEFAULT 'member',
  `bio`          text COMMENT 'free-form profile text',
  `email_domain` varchar(64) GENERATED ALWAYS AS (substring_index(`email`,'@',-1)) STORED,
  `created_at`   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `users_email_uniq` (`email`),
  KEY `users_role_idx` (`role`),
  FULLTEXT KEY `users_bio_ft` (`bio`)
) ENGINE=InnoDB AUTO_INCREMENT=101 DEFAULT CHARSET=utf8mb4 COMMENT='One row per person who can sign in.';

DROP TABLE IF EXISTS `posts`;
CREATE TABLE `posts` (
  `id`          bigint unsigned NOT NULL AUTO_INCREMENT,
  `user_id`     bigint unsigned NOT NULL COMMENT 'author',
  `slug`        varchar(190) NOT NULL,
  `title`       varchar(255) NOT NULL,
  `published_at` datetime DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `posts_slug_uniq` (`slug`),
  KEY `posts_user_idx` (`user_id`),
  CONSTRAINT `posts_user_fk` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

DROP TABLE IF EXISTS `post_tags`;
CREATE TABLE `post_tags` (
  `post_id` bigint unsigned NOT NULL,
  `tag`     varchar(64)     NOT NULL,
  PRIMARY KEY (`post_id`,`tag`),
  CONSTRAINT `post_tags_post_fk` FOREIGN KEY (`post_id`) REFERENCES `posts` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

LOCK TABLES `post_tags` WRITE;
UNLOCK TABLES;

ALTER TABLE `posts` ADD KEY `posts_published_idx` (`published_at`);
