use crate::{
    core::{DB, macros::get_conn},
    models::{Beatmapsets, ChannelKind, Subscriptions},
    schema::{
        self, beatmapset_subscriptions::dsl::beatmapset_subscriptions,
        beatmapsets::dsl::beatmapsets, subscriptions::dsl::subscriptions,
    },
};
use anyhow::Result;
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;

macro_rules! load_subscription {
    ($matchable:ident, [$($kind:ident),*]) => {
        {
            let conn = get_conn!();
            match $matchable {
                $(
                    ChannelType::$kind(_) => {
                    subscriptions
                        .filter(schema::subscriptions::kind.eq(ChannelKind::$kind))
                        .load::<Subscriptions>(conn)
                        .await?
                    }
                )*
            }
        }
    };
}
macro_rules! set_subscription {
    ($channel:expr, $matchable:ident, [$($kind:ident),*]) => {
        let conn = get_conn!();
        let (kind, mode) = match $matchable {
            $(
                ChannelType::$kind(mode) => (ChannelKind::$kind, mode),
            )*
        };
        match mode {
            SubscriptionMode::Subscribe => {
                diesel::insert_into(subscriptions)
                    .values((
                        schema::subscriptions::channel_id.eq($channel),
                        schema::subscriptions::kind.eq(kind),
                    ))
                    .execute(conn)
                    .await?
            }
            SubscriptionMode::Unsubscribe => {
                diesel::delete(subscriptions)
                    .filter(schema::subscriptions::channel_id.eq($channel))
                    .filter(schema::subscriptions::kind.eq(kind))
                    .execute(conn)
                    .await?
            }
        };
    };
}

pub enum SubscriptionMode {
    Subscribe,
    Unsubscribe,
}

pub enum ChannelType {
    Mapfeed(SubscriptionMode),
    Music(SubscriptionMode),
    Groups(SubscriptionMode),
}

pub async fn channel_subscription_handler(channel_id: i64, type_: ChannelType) -> Result<()> {
    set_subscription!(channel_id, type_, [Mapfeed, Music, Groups]);
    Ok(())
}

pub async fn fetch_all_subscribed_channels(type_: ChannelType) -> Result<Vec<i64>> {
    let rows = load_subscription!(type_, [Mapfeed, Music, Groups]);
    Ok(rows.iter().map(|r| r.channel_id).collect::<Vec<i64>>())
}

pub async fn beatmap_subscription_handler(
    user_id: i64,
    beatmap_id: i32,
    type_: SubscriptionMode,
) -> Result<()> {
    let conn = get_conn!();

    match type_ {
        SubscriptionMode::Subscribe => {
            let beatmap = beatmapsets
                .filter(schema::beatmapsets::id.eq(&beatmap_id))
                .select(Beatmapsets::as_select())
                .get_result(conn)
                .await?;
            diesel::insert_into(beatmapset_subscriptions)
                .values((
                    schema::beatmapset_subscriptions::user_id.eq(&user_id),
                    schema::beatmapset_subscriptions::beatmapset_id.eq(beatmap.id),
                ))
                .execute(conn)
                .await?;
        }
        SubscriptionMode::Unsubscribe => {
            diesel::delete(beatmapset_subscriptions)
                .filter(schema::beatmapset_subscriptions::user_id.eq(&user_id))
                .filter(schema::beatmapset_subscriptions::beatmapset_id.eq(&beatmap_id))
                .execute(conn)
                .await?;
        }
    }
    Ok(())
}
