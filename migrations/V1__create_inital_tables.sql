CREATE TABLE beatmapsets (
    beatmapset_id integer PRIMARY KEY,
    subscribed_user_ids bigint[]
);

CREATE TABLE subscribed_channels (
    channel_id bigint PRIMARY KEY
)

