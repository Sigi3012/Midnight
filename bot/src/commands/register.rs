use crate::{Context, Error};
use log::info;

#[poise::command(prefix_command, owners_only)]
pub async fn sync(ctx: Context<'_>) -> Result<(), Error> {
    info!("Attempting to send register modal");
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
