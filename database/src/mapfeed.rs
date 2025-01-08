use crate::{
    core::{DB, macros::get_conn},
    models::{BeatmapsetSubscriptions, Beatmapsets},
    schema::{
        self, beatmapset_subscriptions::dsl::beatmapset_subscriptions,
        beatmapsets::dsl::beatmapsets,
    },
};
use anyhow::Result;
use diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;

pub async fn insert_beatmaps(ids: Vec<i32>) -> Result<()> {
    let ids = ids
        .iter()
        .map(|i| schema::beatmapsets::id.eq(*i))
        .collect::<Vec<_>>();

    diesel::insert_into(beatmapsets)
        .values(ids)
        .execute(get_conn!())
        .await?;
    Ok(())
}

pub async fn delete_beatmap(beatmapset_id: i32) -> Result<()> {
    diesel::delete(beatmapsets.filter(schema::beatmapsets::id.eq(&beatmapset_id)))
        .execute(get_conn!())
        .await?;
    Ok(())
}

pub async fn fetch_all_tracked() -> Result<Option<Vec<i32>>> {
    let rows = beatmapsets.load::<Beatmapsets>(get_conn!()).await?;

    if !rows.is_empty() {
        return Ok(Some(rows.iter().map(|b| b.id).collect::<Vec<_>>()));
    }
    Ok(None)
}

pub async fn fetch_all_subscribers_for_beatmap(beatmapset_id: i32) -> Result<Option<Vec<i64>>> {
    let beatmap = beatmapsets
        .filter(schema::beatmapsets::id.eq(&beatmapset_id))
        .select(Beatmapsets::as_select())
        .get_result(get_conn!())
        .await?;

    let subscribers = BeatmapsetSubscriptions::belonging_to(&beatmap)
        .select(BeatmapsetSubscriptions::as_select())
        .load(get_conn!())
        .await?;

    if !subscribers.is_empty() {
        return Ok(Some(
            subscribers.iter().map(|s| s.user_id).collect::<Vec<_>>(),
        ));
    }
    Ok(None)
}

pub async fn fetch_all_subscriptions_for_user(user_id: i64) -> Result<Option<Vec<i32>>> {
    let beatmaps = beatmapset_subscriptions
        .filter(schema::beatmapset_subscriptions::user_id.eq(&user_id))
        .select(BeatmapsetSubscriptions::as_select())
        .load(get_conn!())
        .await?;

    if !beatmaps.is_empty() {
        return Ok(Some(
            beatmaps.iter().map(|b| b.beatmapset_id).collect::<Vec<_>>(),
        ));
    }
    Ok(None)
}
