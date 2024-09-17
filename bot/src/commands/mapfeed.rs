use crate::{Context, Error};
use backend::{
    api::osu::fetch_beatmaps,
    mapfeed::{create_reply_with_sorted_beatmaps, subscription_handler, unsubscription_handler},
};
use database::mapfeed::fetch_all_subscribed_beatmaps_for_id;
use log::{error, info};
use poise::CreateReply;

#[poise::command(
    slash_command,
    subcommands("subscribe", "unsubscribe", "viewsubscribed", "controls::channel")
)]
pub async fn mapfeed(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Subscribe to a beatmap
#[poise::command(slash_command, category = "Mapfeed")]
pub async fn subscribe(
    ctx: Context<'_>,
    #[description = "The url to the beatmap to subscribe to"] link: String,
) -> Result<(), Error> {
    match subscription_handler(ctx.author().id.get() as i64, link).await {
        Ok(_) => {
            let builder = CreateReply::default()
                .content("Subscribed successfully")
                .ephemeral(true);

            ctx.send(builder).await?;

            info!(
                "Subscribed user {} using subscription command successfully",
                ctx.author().tag(),
            )
        }

        Err(e) => {
            CreateReply::default()
                .content("Something went wrong")
                .ephemeral(true);
            error!(
                "Something went wrong while using beatmap subscription command: {}",
                e
            )
        }
    };
    Ok(())
}

/// Unsubscribe from a beatmap
#[poise::command(slash_command, category = "Mapfeed")]
pub async fn unsubscribe(
    ctx: Context<'_>,
    #[description = "The url to the beatmap to subscribe to"] link: String,
) -> Result<(), Error> {
    match unsubscription_handler(ctx.author().id.get() as i64, link).await {
        Ok(_) => {
            let builder = CreateReply::default()
                .content("Unsubscribed successfully")
                .ephemeral(true);
            ctx.send(builder).await?;

            info!(
                "Unsubscribed user {} using subscription command successfully",
                ctx.author().tag(),
            )
        }

        Err(e) => {
            CreateReply::default()
                .content("Something went wrong")
                .ephemeral(true);
            error!(
                "Something went wrong while using beatmap subscription command: {}",
                e
            )
        }
    };

    Ok(())
}

/// View all beatmaps you are subscribed to
#[poise::command(slash_command, category = "Mapfeed")]
pub async fn viewsubscribed(ctx: Context<'_>) -> Result<(), Error> {
    let builder: CreateReply;
    match fetch_all_subscribed_beatmaps_for_id(ctx.author().id.get() as i64).await? {
        Some(ids) => match fetch_beatmaps(ids).await {
            Ok(beatmaps) => builder = create_reply_with_sorted_beatmaps(beatmaps),
            Err(e) => {
                error!("Something went wrong while fetching beatmapsets: {}", e);
                builder = CreateReply::default()
                    .content("Something went wrong")
                    .ephemeral(true);
            }
        },
        None => {
            builder = CreateReply::default()
                .content("You are not subscribed to any beatmaps")
                .ephemeral(true);
        }
    };

    ctx.send(builder).await?;
    Ok(())
}

mod controls {
    use crate::{Context, Error};
    use database::subscriptions::{subscription_handler, ChannelType, SubscriptionMode};
    use log::info;
    use poise::{reply::CreateReply, serenity_prelude::Mentionable};

    #[poise::command(slash_command, subcommands("subscribe", "unsubscribe"))]
    pub async fn channel(_: Context<'_>) -> Result<(), Error> {
        Ok(())
    }

    /// Subscribeds a channel to the osu! mapfeed
    #[poise::command(
        slash_command,
        category = "Mod",
        required_permissions = "ADMINISTRATOR"
    )]
    pub async fn subscribe(ctx: Context<'_>) -> Result<(), Error> {
        subscription_handler(
            ctx.channel_id().get() as i64,
            ChannelType::Mapfeed(SubscriptionMode::Subscribe),
        )
        .await?;
        info!(
            "Subscribed channel ID: {}, to mapfeed successfully",
            ctx.channel_id().get()
        );
        CreateReply::default()
            .content(format!(
                "Subscribed {} to mapfeed successfully",
                ctx.channel_id().mention()
            ))
            .ephemeral(true);
        Ok(())
    }

    /// Unsubscribeds a channel from the osu! mapfeed
    #[poise::command(
        slash_command,
        category = "Mod",
        required_permissions = "ADMINISTRATOR"
    )]
    pub async fn unsubscribe(ctx: Context<'_>) -> Result<(), Error> {
        subscription_handler(
            ctx.channel_id().get() as i64,
            ChannelType::Mapfeed(SubscriptionMode::Unsubscribe),
        )
        .await?;
        info!(
            "Unsubscribed channel ID: {}, from mapfeed successfully",
            ctx.channel_id().get()
        );
        CreateReply::default()
            .content(format!(
                "Unsubscribed {} from mapfeed successfully",
                ctx.channel_id().mention()
            ))
            .ephemeral(true);

        Ok(())
    }
}
