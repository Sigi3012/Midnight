CREATE TYPE channel_type AS ENUM ('mapfeed', 'music');

ALTER TABLE subscribed_channels
  RENAME TO subscriptions;

ALTER TABLE subscriptions
  DROP CONSTRAINT subscribed_channels_pkey;

ALTER TABLE subscriptions
  ADD COLUMN id SERIAL PRIMARY KEY,
  ADD COLUMN channel_type channel_type;

UPDATE subscriptions SET channel_type = 'mapfeed';
