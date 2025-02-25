use crate::context::Context;
use crate::{DiscordContext, Error};
use log::{error, info};
use poise::{CreateReply, serenity_prelude as serenity};

async fn handle_message_creation(
    ctx: DiscordContext<'_>,
    links_tuple: (String, Option<String>),
) -> Result<(), Error> {
    match links_tuple.1 {
        Some(source_link) => {
            let reply = {
                let components = vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new_link(source_link).label("Original post"),
                ])];

                CreateReply::default()
                    .reply(true)
                    .content(links_tuple.0)
                    .components(components)
            };

            ctx.send(reply).await?;
        }
        None => {
            ctx.reply(links_tuple.0).await?;
        }
    }
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn yuri(ctx: DiscordContext<'_>, count: Option<i32>) -> Result<(), Error> {
    let count = count.unwrap_or(1);

    if count > 5 {
        ctx.reply("Please provide an amount of 5 or lower").await?;
        return Ok(());
    }

    match Context::fun().get_random_booru_post(count).await {
        Ok(links) => {
            info!("Attempting to send {} links", links.len());
            for link in links {
                handle_message_creation(ctx, link).await?;
            }
        }
        Err(e) => {
            ctx.reply("Something went wrong").await?;
            error!(
                "Something went wrong while fetching safebooru posts, {:?}",
                e
            )
        }
    };

    Ok(())
}
