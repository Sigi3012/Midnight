-- Your SQL goes here
CREATE TYPE channel_kind AS ENUM ('mapfeed', 'music');

CREATE TABLE beatmapsets
(
    id INTEGER PRIMARY KEY
);

CREATE TABLE beatmapset_subscriptions
(
    id            SERIAL PRIMARY KEY,
    user_id       BIGINT  NOT NULL,
    beatmapset_id INTEGER NOT NULL REFERENCES beatmapsets (id)
);

CREATE TABLE subscriptions
(
    channel_id BIGINT PRIMARY KEY,
    kind       channel_kind NOT NULL
)