-- This file should undo anything in `up.sql`
CREATE TYPE channel_kind_new AS ENUM ('mapfeed', 'music');

DELETE
FROM subscriptions
WHERE kind = 'groups';

ALTER TABLE subscriptions
    ALTER COLUMN kind TYPE channel_kind_new USING (kind::text::channel_kind_new);

DROP TYPE channel_kind;
ALTER TYPE channel_kind_new RENAME TO channel_kind;