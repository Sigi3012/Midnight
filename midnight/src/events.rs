use crate::{
    Data, Error,
    features::{
        links,
        music::{DownloadError, music_link_handler},
        sticky::sticky_message_handler,
    },
};
use poise::serenity_prelude::{
    self as serenity, CreateAttachment, CreateMessage, FullEvent, Message, MessageFlags,
};
use tracing::{error, info, warn};

pub async fn listener(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            info!("Logged in as {}", data_about_bot.user.name);
        }
        FullEvent::Message { new_message, .. } => handle_incoming_message(ctx, new_message).await?,
        _ => {}
    }

    Ok(())
}

async fn handle_incoming_message(
    ctx: &serenity::Context,
    new_message: &Message,
) -> Result<(), Error> {
    match links::fix_links(new_message).await {
        Ok(result) => {
            if let Some(content) = result {
                let mut target: &Message = new_message;
                if let Some(reply_handle) = &new_message.referenced_message {
                    target = reply_handle
                }
                if let Err(why) = links::message_handler(
                    content,
                    new_message.author.id.get(),
                    new_message.channel_id,
                    target,
                )
                .await
                {
                    error!("Something went wrong while sending reply message: {}", why);
                    return Ok(());
                }

                new_message.delete(ctx).await?;
                // TODO analytics
                info!("Fixed up a message successfully")
            }
        }
        Err(e) => {
            new_message.reply(ctx, "Something went wrong").await?;
            error!(new_message.content, "Failed to fix a link, {}", e)
        }
    };

    match music_link_handler(new_message).await {
        Ok(Some(song)) => {
            let builder = CreateMessage::new()
                .add_file(CreateAttachment::bytes(song.get(), "audio.ogg"))
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
                .reference_message(new_message);

            new_message.channel_id.send_message(&ctx, builder).await?;
        }
        Ok(None) => (),
        Err(DownloadError::FileTooLarge) => {
            warn!("File exceeds upload size");
            new_message
                .reply(&ctx, "File is too large to download")
                .await?;
        }
        Err(DownloadError::DownloadTimeout(e)) => {
            warn!("Download took too long, {}", e);
            new_message.reply(&ctx, "Download timed out").await?;
        }
        Err(_) => {
            new_message.reply(&ctx, "Failed to download audio").await?;
        }
    };

    if let Err(why) = sticky_message_handler(new_message).await {
        error!("Something went wrong while sending sticky message: {}", why)
    }

    Ok(())
}
