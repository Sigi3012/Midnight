use crate::{Context, Error};
use backend::{
    api::osu::fetch_beatmaps,
    mapfeed::{create_reply_with_sorted_beatmaps, subscription_handler},
};
use database::{mapfeed::fetch_all_subscriptions_for_user, subscriptions::SubscriptionMode};
use log::{error, info};
use poise::CreateReply;

#[poise::command(
    slash_command,
    subcommands("subscribe", "unsubscribe", "view_subscribed")
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
    match subscription_handler(
        ctx.author().id.get() as i64,
        &link,
        SubscriptionMode::Subscribe,
    )
    .await
    {
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
    match subscription_handler(
        ctx.author().id.get() as i64,
        &link,
        SubscriptionMode::Unsubscribe,
    )
    .await
    {
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
pub async fn view_subscribed(ctx: Context<'_>) -> Result<(), Error> {
    let builder: CreateReply;
    match fetch_all_subscriptions_for_user(ctx.author().id.get() as i64).await? {
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
