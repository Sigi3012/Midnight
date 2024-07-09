use crate::{Context, Error};
use database::mapfeed::{subscribe_channel_to_mapfeed, unsubscribe_channel_from_mapfeed};
use log::info;
use poise::{reply::CreateReply, serenity_prelude::Mentionable};

#[poise::command(slash_command, subcommands("subscribe", "unsubscribe"))]
pub async fn mapfeed(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn subscribe(ctx: Context<'_>) -> Result<(), Error> {
    subscribe_channel_to_mapfeed(ctx.channel_id().get() as i64).await?;
    info!(
        "Subscribed channel ID: {}, to mapfeed successfully",
        ctx.channel_id().get()
    );
    CreateReply::default().content(format!(
        "Subscribed {} to mapfeed successfully",
        ctx.channel_id().mention()
    ));
    Ok(())
}

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn unsubscribe(ctx: Context<'_>) -> Result<(), Error> {
    unsubscribe_channel_from_mapfeed(ctx.channel_id().get() as i64).await?;
    info!(
        "Unsubscribed channel ID: {}, from mapfeed successfully",
        ctx.channel_id().get()
    );
    CreateReply::default().content(format!(
        "Unsubscribed {} from mapfeed successfully",
        ctx.channel_id().mention()
    ));

    Ok(())
}
