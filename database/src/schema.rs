// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "channel_kind"))]
    pub struct ChannelKind;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "osu_gamemode"))]
    pub struct OsuGamemode;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "osu_group"))]
    pub struct OsuGroup;
}

diesel::table! {
    beatmapset_subscriptions (id) {
        id -> Int4,
        user_id -> Int8,
        beatmapset_id -> Int4,
    }
}

diesel::table! {
    beatmapsets (id) {
        id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::OsuGamemode;

    osu_user_group_gamemodes (id) {
        id -> Int4,
        user_group_id -> Int4,
        gamemode -> OsuGamemode,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::OsuGroup;

    osu_user_groups (id) {
        id -> Int4,
        user_id -> Int4,
        member_of -> OsuGroup,
    }
}

diesel::table! {
    osu_users (id) {
        id -> Int4,
        username -> Text,
        avatar_url -> Text,
    }
}

diesel::table! {
    sticky_messages (id) {
        id -> Int4,
        channel_id -> Int8,
        orig_message_id -> Int8,
        bot_message_id -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ChannelKind;

    subscriptions (channel_id) {
        channel_id -> Int8,
        kind -> ChannelKind,
    }
}

diesel::joinable!(beatmapset_subscriptions -> beatmapsets (beatmapset_id));
diesel::joinable!(osu_user_group_gamemodes -> osu_user_groups (user_group_id));
diesel::joinable!(osu_user_groups -> osu_users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    beatmapset_subscriptions,
    beatmapsets,
    osu_user_group_gamemodes,
    osu_user_groups,
    osu_users,
    sticky_messages,
    subscriptions,
);
