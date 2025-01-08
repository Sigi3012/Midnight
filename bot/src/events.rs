use crate::{Data, Error};
use backend::{
    links,
    music::{DownloadError, music_link_handler},
};
use log::{error, info, warn};
use poise::serenity_prelude::{
    self as serenity, CreateAttachment, CreateMessage, Message, MessageFlags,
};

pub async fn listener(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            info!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::Message { new_message, .. } => {
            match links::fix_links(new_message).await {
                Ok(result) => {
                    if let Some(content) = result {
                        let mut target: &Message = new_message;
                        if let Some(reply_handle) = &new_message.referenced_message {
                            target = reply_handle
                        }
                        match links::message_handler(
                            content,
                            new_message.author.id.get(),
                            new_message.channel_id,
                            target,
                        )
                        .await
                        {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Something went wrong while sending reply message: {}", e);
                                return Ok(());
                            }
                        }
                        new_message.delete(ctx).await?;
                        // TODO analytics
                        info!("Fixed up a message successfully")
                    }
                }
                Err(e) => {
                    new_message.reply(ctx, "Something went wrong").await?;
                    error!("Something went wrong while fixing a link! {}", e)
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
            }
        }
        _ => {}
    }

    Ok(())
}
