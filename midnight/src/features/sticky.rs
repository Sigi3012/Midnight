use crate::context::Context;
use anyhow::{Result, bail};
use midnight_database::sticky::{self, untrack_message};
use poise::serenity_prelude::{Message, MessageType};
use smallvec::SmallVec;
use tracing::{debug, trace, warn};

pub async fn sticky_message_handler(message: &Message) -> Result<()> {
    let ctx = Context::discord_ctx();

    if message.kind != MessageType::PinsAdd {
        return Ok(());
    };
    if message.author.bot {
        warn!("Deleting bot pinned system message");
        message.delete(&ctx).await?;
        return Ok(());
    }

    let Some(ref_message) = &message.message_reference else {
        trace!("{message:?}");
        bail!("Could not find the referenced message")
    };
    let channel_id = ref_message.channel_id;

    let mut conn = Context::database().get_conn().await;
    match sticky::check_channel(&mut conn, ref_message.channel_id.get() as i64).await {
        Ok(v) if v.is_empty() => Ok(()),
        Ok(tracked) => {
            debug!("{tracked:?}");
            let mut v: SmallVec<[Message; 4]> = SmallVec::new();
            for message in tracked {
                v.push(
                    channel_id
                        .message(&ctx, message.bot_message_id as u64)
                        .await?,
                )
            }
            for message in v {
                if !message.pinned {
                    untrack_message(&mut conn, message.id.get() as i64).await?;
                    message.delete(&ctx).await?;
                    continue;
                }
                message.unpin(&ctx).await?;
                message.pin(&ctx).await?
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
