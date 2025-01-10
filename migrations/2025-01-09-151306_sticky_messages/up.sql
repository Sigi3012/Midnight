-- Your SQL goes here
CREATE TABLE sticky_messages
(
    id SERIAL PRIMARY KEY,
    channel_id BIGINT NOT NULL,
    orig_message_id BIGINT NOT NULL,
    bot_message_id BIGINT NOT NULL
)