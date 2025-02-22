use crate::{Context, Error};
use backend::music;
use database::subscriptions::{ChannelType, SubscriptionMode, channel_subscription_handler};
use paste::paste;
use poise::{reply::CreateReply, serenity_prelude::Mentionable};
use tracing::info;

#[poise::command(
    slash_command,
    rename = "mod",
    subcommands("mapfeed", "music", "group")
)]
pub async fn _mod(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, subcommands("mapfeed_subscribe", "mapfeed_unsubscribe"))]
pub async fn mapfeed(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, subcommands("music_subscribe", "music_unsubscribe"))]
pub async fn music(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, subcommands("groups_subscribe", "groups_unsubscribe"))]
pub async fn group(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

macro_rules! construct_commands {
    ($ident:ident, $middleware:expr, $help_text:literal) => {
        paste! {
            #[doc = "Subscribes a channel to " $help_text]
            #[poise::command(
                slash_command,
                rename = "subscribe",
                category = "Mod",
                required_permissions = "ADMINISTRATOR"
            )]
            pub async fn [<$ident:lower _subscribe>](ctx: Context<'_>) -> Result<(), Error> {
                channel_subscription_handler(
                    ctx.channel_id().get() as i64,
                    ChannelType::$ident(SubscriptionMode::Subscribe)
                ).await?;
                $middleware
                info!("Subscribed channel ID: {} to {} successfully", ctx.id(), $help_text);
                let builder = CreateReply::default()
                    .content(format!("Subscribed {} to {} successfully", ctx.channel_id().mention(), $help_text))
                    .ephemeral(true);
                ctx.send(builder).await?;

                Ok(())
            }
            #[doc = "Unsubscribes a channel from" $help_text]
            #[poise::command(
                slash_command,
                rename = "unsubscribe",
                category = "Mod",
                required_permissions = "ADMINISTRATOR"
            )]
            pub async fn [<$ident:lower _unsubscribe>](ctx: Context<'_>) -> Result<(), Error> {
                channel_subscription_handler(
                    ctx.channel_id().get() as i64,
                    ChannelType::$ident(SubscriptionMode::Unsubscribe),
                )
                .await?;
                $middleware
                info!("Unsubscribed channel ID: {}, from {} successfully", ctx.channel_id().get(), $help_text);
                let builder = CreateReply::default()
                    .content(format!("Unsubscribed {} from {} successfully", ctx.channel_id().mention(), $help_text))
                    .ephemeral(true);
                ctx.send(builder).await?;

                Ok(())
            }
        }
    };
}

construct_commands!(Mapfeed, {}, "osu! mapfeed");
construct_commands!(
    Music,
    {
        music::CHANNEL_CACHE.update_cache().await;
    },
    "music downloader"
);
construct_commands!(Groups, {}, "group tracker");
