use crate::{Data, DiscordContext, Error, context::Context};
use midnight_database::sticky;
use midnight_model::database::NewStickyMessage;
use midnight_util::constants::STICKY_ERROR_MESSAGE;
use poise::{
    CreateReply, FrameworkError,
    serenity_prelude::{
        self as serenity, ChannelId, Colour, CreateEmbed, CreateEmbedFooter, CreateMessage,
    },
};
use tracing::{error, warn};

/// Toggles whether a selected message is sticky pinned
#[poise::command(
    slash_command,
    context_menu_command = "Toggle sticky pin this message",
    guild_only,
    required_permissions = "MANAGE_MESSAGES",
    on_error = "error_handler"
)]
pub async fn sticky(
    ctx: DiscordContext<'_>,
    #[description = "A link to the message you want to sticky pin"] message: serenity::Message,
) -> Result<(), Error> {
    let mut conn = Context::database().get_conn().await;

    let store = sticky::check_channel(&mut conn, message.channel_id.get() as i64).await?;
    let message_id = message.id.get() as i64;

    match store
        .iter()
        .find(|item| item.orig_message_id == message_id || item.bot_message_id == message_id)
    {
        Some(tracked) => {
            let channel_id = ChannelId::new(tracked.channel_id as u64);
            channel_id
                .message(&ctx, tracked.bot_message_id as u64)
                .await?
                .delete(&ctx)
                .await?;
            sticky::untrack_message(&mut conn, tracked.orig_message_id).await?;
            ctx.send(
                CreateReply::default()
                    .content("Sticky message removed.")
                    .ephemeral(true),
            )
            .await?;
        }
        None => {
            message.unpin(&ctx).await?;

            let embed = CreateEmbed::default()
                .title("Sticky Message")
                .description(format!(
                    "\"{}\"\n{}",
                    message.content_safe(ctx),
                    message.link_ensured(&ctx).await
                ))
                .color(Colour::new(0xffee8c))
                .timestamp(message.timestamp)
                .footer(
                    CreateEmbedFooter::new(&message.author.name)
                        .icon_url(message.author.avatar_url().unwrap_or_default()),
                );
            let bot_message = message
                .channel_id
                .send_message(&ctx, CreateMessage::new().embed(embed))
                .await?;
            bot_message.pin(&ctx).await?;

            sticky::track_message(
                &mut conn,
                NewStickyMessage {
                    channel_id: message.channel_id.into(),
                    orig_message_id: message.id.into(),
                    bot_message_id: bot_message.id.into(),
                },
            )
            .await?;
            ctx.send(
                CreateReply::default()
                    .content("Sticky message added.")
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

async fn error_handler(error: FrameworkError<'_, Data, Error>) {
    match error {
        FrameworkError::ArgumentParse {
            error, input, ctx, ..
        } => {
            warn!(error, input);
            if let Err(why) = ctx
                .send(
                    CreateReply::default()
                        .content(STICKY_ERROR_MESSAGE)
                        .ephemeral(true),
                )
                .await
            {
                error!("Failed to send error message: {}", why);
            }
        }
        _ => {
            if let Err(why) = poise::builtins::on_error(error).await {
                error!("Builtin error handler failed: {why}")
            };
        }
    }
}
