use crate::{Data, DiscordContext, Error, context::Context};
use poise::{
    CreateReply,
    serenity_prelude::{Colour, CreateEmbed},
};
use tokio::time::Instant;

impl Data {
    pub fn uptime(&self) -> String {
        let duration = Instant::now().duration_since(*Context::startup_time());
        let seconds = duration.as_secs();

        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;
        let remaining_seconds = seconds % 6;

        Self::format_duration(days, hours, minutes, remaining_seconds)
    }

    fn format_duration(days: u64, hours: u64, minutes: u64, seconds: u64) -> String {
        match (days, hours, minutes) {
            (0, 0, 0) => format!("{} seconds", seconds),
            (0, 0, _) => format!("{} minutes, {} seconds", minutes, seconds),
            (0, _, _) => format!("{} hours, {} minutes, {} seconds", hours, minutes, seconds),
            (_, _, _) => format!(
                "{} days, {} hours, {} minutes, {} seconds",
                days, hours, minutes, seconds
            ),
        }
    }
}

/// Shows various informationals about the bot
#[poise::command(prefix_command, category = "Utility")]
pub async fn status(ctx: DiscordContext<'_>) -> Result<(), Error> {
    let start = Instant::now();
    let message = ctx.say("Loading..").await?;

    let bot_latency = start.elapsed().as_millis();
    let shard_latency = ctx.ping().await.as_millis();

    // TODO Mapfeed health check
    let description = format!(
        "**Mapfeed health:** {}\n**Uptime:** {}",
        "Work in progress",
        ctx.data().uptime()
    );

    let fields = vec![(
        "Ping",
        format!(
            "**Shard Latency:** `{}ms`\n\
                             The time it takes for Discord to ping the shard.\n\
                             **Response/API Latency:** `{}ms`\n\
                             The time it takes for me to ping Discord.",
            shard_latency, bot_latency
        ),
        false,
    )];

    let builder = CreateReply::default().content("").embed(
        CreateEmbed::default()
            .title("Status")
            .description(description)
            .colour(Colour::new(0xfc4fca))
            .fields(fields),
    );

    message.edit(ctx, builder).await?;

    Ok(())
}
