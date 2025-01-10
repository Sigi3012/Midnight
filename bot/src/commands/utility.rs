use crate::{Context, Data, Error};
use common::sys::SYSTEM;
use poise::{
    CreateReply,
    serenity_prelude::{Colour, CreateEmbed},
};
use tokio::time::Instant;

const KILOBYTE: u64 = 1024;
const MEGABYTE: u64 = KILOBYTE * 1024;
const GIGABYTE: u64 = MEGABYTE * 1024;

impl Data {
    pub fn uptime(&self) -> String {
        let duration = Instant::now().duration_since(self.startup_time);
        let seconds = duration.as_secs();

        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;
        let remaining_seconds = seconds % 60;

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

fn format_bytes(bytes: u64) -> String {
    let (value, unit) = match bytes {
        b if b >= GIGABYTE => (bytes as f64 / GIGABYTE as f64, "GB"),
        b if b >= MEGABYTE => (bytes as f64 / MEGABYTE as f64, "MB"),
        b if b >= KILOBYTE => (bytes as f64 / KILOBYTE as f64, "KB"),
        _ => (bytes as f64, "B"),
    };
    format!("{:.2} {}", value, unit)
}

/// Shows various informationals about the bot
#[poise::command(prefix_command, category = "Utility")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let start = Instant::now();
    let message = ctx.say("Loading..").await?;

    let bot_latency = start.elapsed().as_millis();
    let shard_latency = ctx.ping().await.as_millis();

    let (
        system_cpu_usage,
        system_ram_usage,
        system_ram_total,
        system_swap_usage,
        system_swap_total,
    ) = {
        let mut guard = SYSTEM.lock().await;
        guard.refresh_cpu_usage();
        guard.refresh_memory();

        (
            guard.global_cpu_usage(),
            guard.used_memory(),
            guard.total_memory(),
            guard.used_swap(),
            guard.total_memory(),
        )
    };

    // TODO Mapfeed health check
    let description = format!(
        "**Mapfeed health:** {}\n**Uptime:** {}",
        "Work in progress",
        ctx.data().uptime()
    );

    let fields = vec![
        (
            "Ping",
            format!(
                "**Shard Latency:** `{}ms`\n\
                             The time it takes for Discord to ping the shard.\n\
                             **Response/API Latency:** `{}ms`\n\
                             The time it takes for me to ping Discord.",
                shard_latency, bot_latency
            ),
            false,
        ),
        (
            "System",
            format!(
                "CPU usage: `{:.2}%`\nRAM usage: `{} / {}`\nSwap usage: `{} / {}`",
                system_cpu_usage,
                format_bytes(system_ram_usage),
                format_bytes(system_ram_total),
                format_bytes(system_swap_usage),
                format_bytes(system_swap_total)
            ),
            false,
        ),
    ];

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
