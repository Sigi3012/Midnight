use crate::{Data, Error};
use backend::links;
use log::{error, info};
use poise::serenity_prelude as serenity;

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
                Ok(result) => match result {
                    Some(content) => {
                        new_message.reply_mention(ctx, content).await?;
                        new_message.delete(ctx).await?;
                        // TODO analytics
                        info!("Fixed up a message successfully")
                    }
                    // No links were fixed ignore..
                    None => (),
                },
                Err(e) => {
                    new_message.reply(ctx, "Something went wrong").await?;
                    error!("Something went wrong while fixing a link! {}", e)
                }
            };
        }
        _ => {}
    }

    Ok(())
}
