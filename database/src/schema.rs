// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "channel_kind"))]
    pub struct ChannelKind;
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
    use super::sql_types::ChannelKind;

    subscriptions (channel_id) {
        channel_id -> Int8,
        kind -> ChannelKind,
    }
}

diesel::joinable!(beatmapset_subscriptions -> beatmapsets (beatmapset_id));

diesel::allow_tables_to_appear_in_same_query!(beatmapset_subscriptions, beatmapsets, subscriptions,);
