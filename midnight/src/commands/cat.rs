use crate::context::Context;
use crate::{DiscordContext, Error};
use log::error;
use poise::CreateReply;

#[poise::command(prefix_command)]
pub async fn cat(ctx: DiscordContext<'_>, count: Option<i32>) -> Result<(), Error> {
    let count = count.unwrap_or(1);

    if count > 5 {
        ctx.reply("Please provide an amount of 5 or lower").await?;
        return Ok(());
    }

    match Context::fun().get_random_cat_image(count).await {
        Ok(responses) => {
            for item in responses {
                let content;
                if let Some(breed) = item.breeds {
                    content = format!("Breed: {}\n[image]({})", breed.name, item.url)
                } else {
                    content = item.url
                }
                let builder = CreateReply::default().content(content).reply(true);
                ctx.send(builder).await?;
            }
        }
        Err(e) => {
            ctx.reply("Something went wrong").await?;
            error!("Something went wrong while fetching cat images, {:?}", e)
        }
    };

    Ok(())
}
