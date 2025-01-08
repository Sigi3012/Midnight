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

pub enum SubscriptionMode {
    Subscribe,
    Unsubscribe,
}

pub enum ChannelType {
    Mapfeed(SubscriptionMode),
    Music(SubscriptionMode),
}

pub async fn channel_subscription_handler(channel_id: i64, type_: ChannelType) -> Result<()> {
    let conn = get_conn!();
    let (kind, mode) = match type_ {
        ChannelType::Mapfeed(mode) => (ChannelKind::Mapfeed, mode),
        ChannelType::Music(mode) => (ChannelKind::Music, mode),
    };

    match mode {
        SubscriptionMode::Subscribe => {
            diesel::insert_into(subscriptions)
                .values((
                    schema::subscriptions::channel_id.eq(channel_id),
                    schema::subscriptions::kind.eq(kind),
                ))
                .execute(conn)
                .await?
        }

        SubscriptionMode::Unsubscribe => {
            diesel::delete(subscriptions)
                .filter(schema::subscriptions::channel_id.eq(channel_id))
                .filter(schema::subscriptions::kind.eq(kind))
                .execute(conn)
                .await?
        }
    };
    Ok(())
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

pub async fn fetch_all_subscribed_channels(type_: ChannelType) -> Result<Option<Vec<i64>>> {
    let conn = get_conn!();
    let rows = match type_ {
        ChannelType::Mapfeed(_) => {
            subscriptions
                .filter(schema::subscriptions::kind.eq(ChannelKind::Mapfeed))
                .load::<Subscriptions>(conn)
                .await?
        }
        ChannelType::Music(_) => {
            subscriptions
                .filter(schema::subscriptions::kind.eq(ChannelKind::Music))
                .load::<Subscriptions>(conn)
                .await?
        }
    };

    if !rows.is_empty() {
        return Ok(Some(
            rows.iter().map(|r| r.channel_id).collect::<Vec<i64>>(),
        ));
    }
    Ok(None)
}
