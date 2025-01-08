use crate::{Context, Error};
use backend::music;
use database::subscriptions::{ChannelType, SubscriptionMode, channel_subscription_handler};
use log::info;
use poise::{reply::CreateReply, serenity_prelude::Mentionable};

#[poise::command(slash_command, rename = "mod", subcommands("mapfeed", "music"))]
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

/// Subscribes a channel to the osu! mapfeed
#[poise::command(
    slash_command,
    rename = "subscribe",
    category = "Mod",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn mapfeed_subscribe(ctx: Context<'_>) -> Result<(), Error> {
    channel_subscription_handler(
        ctx.channel_id().get() as i64,
        ChannelType::Mapfeed(SubscriptionMode::Subscribe),
    )
    .await?;
    info!(
        "Subscribed channel ID: {}, to mapfeed successfully",
        ctx.channel_id().get()
    );
    let builder = CreateReply::default()
        .content(format!(
            "Subscribed {} to mapfeed successfully",
            ctx.channel_id().mention()
        ))
        .ephemeral(true);
    ctx.send(builder).await?;

    Ok(())
}

/// Unsubscribes a channel from the osu! mapfeed
#[poise::command(
    slash_command,
    rename = "unsubscribe",
    category = "Mod",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn mapfeed_unsubscribe(ctx: Context<'_>) -> Result<(), Error> {
    channel_subscription_handler(
        ctx.channel_id().get() as i64,
        ChannelType::Mapfeed(SubscriptionMode::Unsubscribe),
    )
    .await?;
    info!(
        "Unsubscribed channel ID: {}, from mapfeed successfully",
        ctx.channel_id().get()
    );
    let builder = CreateReply::default()
        .content(format!(
            "Unsubscribed {} from mapfeed successfully",
            ctx.channel_id().mention()
        ))
        .ephemeral(true);
    ctx.send(builder).await?;

    Ok(())
}

/// Subscribeds a channel to music downloader
#[poise::command(
    slash_command,
    rename = "subscribe",
    category = "Mod",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn music_subscribe(ctx: Context<'_>) -> Result<(), Error> {
    channel_subscription_handler(
        ctx.channel_id().get() as i64,
        ChannelType::Music(SubscriptionMode::Subscribe),
    )
    .await?;
    music::CHANNEL_CACHE.update_cache().await;

    info!(
        "Subscribed channel ID: {}, to music downloader successfully",
        ctx.channel_id().get()
    );
    let builder = CreateReply::default()
        .content(format!(
            "Subscribed {} to music downloader successfully",
            ctx.channel_id().mention()
        ))
        .ephemeral(true);
    ctx.send(builder).await?;

    Ok(())
}

/// Unsubscribes a channel from the music downloader
#[poise::command(
    slash_command,
    rename = "unsubscribe",
    category = "Mod",
    required_permissions = "ADMINISTRATOR"
)]
pub async fn music_unsubscribe(ctx: Context<'_>) -> Result<(), Error> {
    channel_subscription_handler(
        ctx.channel_id().get() as i64,
        ChannelType::Music(SubscriptionMode::Unsubscribe),
    )
    .await?;
    music::CHANNEL_CACHE.update_cache().await;

    info!(
        "Unsubscribed channel ID: {}, from music downloader successfully",
        ctx.channel_id().get()
    );
    let builder = CreateReply::default()
        .content(format!(
            "Unsubscribed {} from music downloader successfully",
            ctx.channel_id().mention()
        ))
        .ephemeral(true);
    ctx.send(builder).await?;

    Ok(())
}
