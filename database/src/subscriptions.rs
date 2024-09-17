use crate::core::{get_client_from_pool, DatabaseError};
use tokio_postgres::types::{FromSql, Type};

const SUBSCRIBE_CHANNEL_TO_MAPFEED_QUERY: &str = r#"
    INSERT INTO subscriptions (channel_id, channel_type) VALUES ($1, 'mapfeed') ON CONFLICT DO NOTHING
"#;

const UNSUBSCRIBE_CHANNEL_FROM_MAPFEED_QUERY: &str = r#"
    DELETE FROM subscriptions WHERE channel_id = $1 AND channel_type = 'mapfeed'
"#;

const SUBSCRIBE_CHANNEL_TO_MUSIC_QUERY: &str = r#"
    INSERT INTO subscriptions (channel_id, channel_type) VALUES ($1, 'music') ON CONFLICT DO NOTHING
"#;

const UNSUBSCRIBE_CHANNEL_FROM_MUSIC_QUERY: &str = r#"
    DELETE FROM subscriptions WHERE channel_id = $1 AND channel_type = 'music'
"#;

const SELECT_ALL_MAPFEED_SUBSCRIBERS_QUERY: &str = r#"
    SELECT * FROM subscriptions WHERE channel_type = 'mapfeed'
"#;

const SELECT_ALL_MUSIC_SUBSCRIBERS_QUERY: &str = r#"
    SELECT * FROM subscriptions WHERE channel_type = 'music'
"#;

pub enum SubscriptionMode {
    Subscribe,
    Unsubscribe,
}

pub enum ChannelType {
    Mapfeed(SubscriptionMode),
    Music(SubscriptionMode),
}

#[derive(Debug, PartialEq)]
pub enum UserAdditionStatus {
    UserAdded,
    UserAlreadyExists,
}

#[derive(Debug, PartialEq)]
pub enum UserDeletionStatus {
    UserRemoved,
    UserDoesNotExist,
}

impl<'a> FromSql<'a> for UserAdditionStatus {
    fn from_sql(
        _type: &Type,
        raw: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        match raw {
            b"UserAdded" => Ok(UserAdditionStatus::UserAdded),
            b"UserAlreadyExists" => Ok(UserAdditionStatus::UserAlreadyExists),
            _ => Err("Unknown user addition status".into()),
        }
    }

    fn accepts(type_: &Type) -> bool {
        type_.name() == "user_addition_status"
    }
}

impl<'a> FromSql<'a> for UserDeletionStatus {
    fn from_sql(
        _type: &Type,
        raw: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        match raw {
            b"UserRemoved" => Ok(UserDeletionStatus::UserRemoved),
            b"UserDoesNotExist" => Ok(UserDeletionStatus::UserDoesNotExist),
            _ => Err("Unknown user addition status".into()),
        }
    }

    fn accepts(type_: &Type) -> bool {
        type_.name() == "user_deletion_status"
    }
}

pub async fn subscription_handler(
    channel_id: i64,
    type_: ChannelType,
) -> Result<(), DatabaseError> {
    let client = get_client_from_pool().await?;

    let sql: &str = match type_ {
        ChannelType::Mapfeed(kind) => match kind {
            SubscriptionMode::Subscribe => SUBSCRIBE_CHANNEL_TO_MAPFEED_QUERY,
            SubscriptionMode::Unsubscribe => UNSUBSCRIBE_CHANNEL_FROM_MAPFEED_QUERY,
        },
        ChannelType::Music(kind) => match kind {
            SubscriptionMode::Subscribe => SUBSCRIBE_CHANNEL_TO_MUSIC_QUERY,
            SubscriptionMode::Unsubscribe => UNSUBSCRIBE_CHANNEL_FROM_MUSIC_QUERY,
        },
    };
    let stmt = client.prepare_cached(sql).await?;
    client.execute(&stmt, &[&channel_id]).await?;
    Ok(())
}

pub async fn fetch_all_subscribed_channels(
    type_: ChannelType,
) -> Result<Option<Vec<i64>>, DatabaseError> {
    let client = get_client_from_pool().await?;

    let sql: &str = match type_ {
        ChannelType::Mapfeed(_) => SELECT_ALL_MAPFEED_SUBSCRIBERS_QUERY,
        ChannelType::Music(_) => SELECT_ALL_MUSIC_SUBSCRIBERS_QUERY,
    };
    let stmt = client.prepare_cached(sql).await?;

    let channel_ids: Vec<tokio_postgres::Row> = client.query(&stmt, &[]).await?;
    if channel_ids.is_empty() {
        return Ok(None);
    }

    Ok(Some(
        channel_ids
            .iter()
            .map(|entry| entry.try_get("channel_id"))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}
