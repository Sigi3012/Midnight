use crate::{
    core::{get_client_from_pool, DatabaseError},
    subscriptions::{UserAdditionStatus, UserDeletionStatus},
};
use log::debug;

const INSERTION_QUERY: &str = r#"
    INSERT INTO beatmapsets (beatmapset_id) VALUES ($1) ON CONFLICT DO NOTHING
"#;

const DELETION_QUERY: &str = r#"
    DELETE FROM beatmapsets WHERE beatmapset_id = $1
"#;

const INSERT_NEW_BEATMAP_SUBSCRIBER_QUERY: &str = r#"
    SELECT add_user_id_to_subscribed_users($1, $2)
"#;

const REMOVE_BEATMAP_SUBSCRIBER_QUERY: &str = r#"
    SELECT remove_user_id_from_subscribed_users($1, $2)
"#;

const SELECT_ALL_QUERY: &str = r#"
    SELECT * FROM beatmapsets
"#;

const SELECT_ALL_SUBSCRIBERS_QUERY: &str = r#"
    SELECT subscribed_user_ids FROM beatmapsets WHERE beatmapset_id = $1
"#;

const SELECT_ALL_SUBSCRIBED_FOR_USER_ID: &str = r#"
    SELECT beatmapset_id FROM beatmapsets WHERE $1 = ANY(subscribed_user_ids)
"#;

pub async fn insert_beatmap(id: i32) -> Result<(), DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client.prepare_cached(INSERTION_QUERY).await?;
    client.execute(&stmt, &[&id]).await?;

    Ok(())
}

pub async fn insert_beatmaps(ids: Vec<i32>) -> Result<(), DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client.prepare_cached(INSERTION_QUERY).await?;

    for id in ids {
        client.execute(&stmt, &[&id]).await?;
        debug!("Inserted Id {} successfully.", id);
    }
    Ok(())
}

pub async fn delete_beatmap(id: i32) -> Result<(), DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client.prepare_cached(DELETION_QUERY).await?;
    client.execute(&stmt, &[&id]).await?;

    Ok(())
}

pub async fn subscribe_to_beatmap(
    user_id: i64,
    beatmap_id: i32,
) -> Result<UserAdditionStatus, DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client
        .prepare_cached(INSERT_NEW_BEATMAP_SUBSCRIBER_QUERY)
        .await?;

    let rows = client.query(&stmt, &[&user_id, &beatmap_id]).await?;
    if let Some(row) = rows.first() {
        let status: UserAdditionStatus = row.get(0);
        Ok(status)
    } else {
        Err(DatabaseError::UnexpectedResult)
    }
}

pub async fn unsubscribe_from_beatmap(
    user_id: i64,
    beatmap_id: i32,
) -> Result<UserDeletionStatus, DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client
        .prepare_cached(REMOVE_BEATMAP_SUBSCRIBER_QUERY)
        .await?;

    let rows = client.query(&stmt, &[&user_id, &beatmap_id]).await?;
    if let Some(row) = rows.first() {
        let status: UserDeletionStatus = row.get(0);
        Ok(status)
    } else {
        Err(DatabaseError::UnexpectedResult)
    }
}

pub async fn fetch_all_ids() -> Result<Option<Vec<i32>>, DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client.prepare_cached(SELECT_ALL_QUERY).await?;

    let rows = client.query(&stmt, &[]).await?;
    if rows.is_empty() {
        return Ok(None);
    }

    let mut return_vec: Vec<i32> = Vec::new();
    for entry in rows {
        return_vec.push(entry.try_get("beatmapset_id")?);
    }

    Ok(Some(return_vec))
}

pub async fn fetch_all_subscribers(primary_key: i32) -> Result<Option<Vec<i64>>, DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client.prepare_cached(SELECT_ALL_SUBSCRIBERS_QUERY).await?;

    let row = client.query_opt(&stmt, &[&primary_key]).await?;

    match row {
        Some(row) => {
            if let Some(ids) = row.get(0) {
                Ok(Some(ids))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

pub async fn fetch_all_subscribed_beatmaps_for_id(
    user_id: i64,
) -> Result<Option<Vec<i32>>, DatabaseError> {
    let client = get_client_from_pool().await?;
    let stmt = client
        .prepare_cached(SELECT_ALL_SUBSCRIBED_FOR_USER_ID)
        .await?;

    let rows = client.query(&stmt, &[&user_id]).await?;
    if rows.is_empty() {
        return Ok(None);
    }

    let return_vec: Vec<i32> = rows
        .iter()
        .map(|r| r.try_get("beatmapset_id"))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(return_vec))
}
