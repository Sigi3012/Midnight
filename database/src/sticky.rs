use crate::{
    core::{DB, macros::get_conn},
    models::{NewStickyMessage, StickyMessages},
    schema::{self, sticky_messages::dsl::sticky_messages},
};
use anyhow::Result;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use futures::{TryStreamExt, future};
use smallvec::SmallVec;
use tracing::{debug, instrument, warn};

#[instrument]
pub async fn track_message(message: NewStickyMessage) -> Result<()> {
    diesel::insert_into(sticky_messages)
        .values(message)
        .execute(get_conn!())
        .await?;
    debug!("Inserted");

    Ok(())
}

#[instrument]
pub async fn untrack_message(message_id: i64) -> Result<()> {
    diesel::delete(sticky_messages)
        .filter(schema::sticky_messages::orig_message_id.eq(message_id))
        .or_filter(schema::sticky_messages::bot_message_id.eq(message_id))
        .execute(get_conn!())
        .await?;
    debug!("Deleted");

    Ok(())
}

/// Finds tracked pinned messages in a channel
///
/// On an incoming pin, pass the channel id to this function
/// to then get a `Vec<StickyMessages>` of stored messages to then re-order the pins
#[instrument]
pub async fn check_channel(channel_id: i64) -> Result<SmallVec<[StickyMessages; 4]>> {
    let messages: SmallVec<[StickyMessages; 4]> = sticky_messages
        .filter(schema::sticky_messages::channel_id.eq(channel_id))
        .load_stream::<StickyMessages>(get_conn!())
        .await?
        .try_fold(SmallVec::new(), |mut acc, item| {
            acc.push(item);
            future::ready(Ok(acc))
        })
        .await?;

    // This will only happen if there are more than four tracked pinned messages
    if messages.spilled() {
        warn!("Ids spilled onto heap, heap items {}", messages.len() - 4)
    };
    Ok(messages)
}
