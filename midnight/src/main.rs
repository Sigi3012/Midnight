use crate::context::DiscordContextWrapper;
use config::CONFIG;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod config;
mod context;
mod events;
mod features;
mod tasks;

pub struct Data;
type Error = Box<dyn std::error::Error + Send + Sync>;
type DiscordContext<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "midnight=info,midnight-api=info,midnight-database=info,midnight-model=info,midnight-util=info,serenity=warn,poise=warn".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MESSAGES;

    let framework_options = poise::FrameworkOptions {
        commands: vec![
            commands::register::sync(),
            commands::yuri::yuri(),
            commands::mapfeed::mapfeed(),
            commands::moderation::_mod(),
            commands::utility::status(),
            commands::cat::cat(),
            commands::sticky::sticky(),
        ],

        event_handler: |ctx, event, framework, data| {
            Box::pin(events::listener(ctx, event, framework, data))
        },
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("-".into()),
            mention_as_prefix: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .options(framework_options)
        .setup(move |ctx, _ready, _framework| {
            Box::pin(async move {
                if let Err(why) = context::Context::new(
                    &CONFIG,
                    DiscordContextWrapper {
                        shard: ctx.shard.clone(),
                        http: ctx.http.clone(),
                        cache: ctx.cache.clone(),
                    },
                )
                .await
                {
                    panic!("Failed to setup application context, {}", why);
                }

                tasks::init_tasks().await;

                Ok(Data {})
            })
        })
        .build();

    let mut client: serenity::Client =
        serenity::ClientBuilder::new(&*CONFIG.tokens.discord, intents)
            .framework(framework)
            .await
            .expect("Client should be creatable");

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        wait_until_shutdown().await;

        warn!("Received Ctrl+C, closing shards..");
        shard_manager.shutdown_all().await;
        info!("All shards closed");
    });

    info!("Starting bot");

    if let Err(err) = client.start_autosharded().await {
        error!("Client error: {}", err)
    }
}

#[cfg(unix)]
#[allow(clippy::unwrap_used)]
async fn wait_until_shutdown() {
    use tokio::signal::unix as signal;

    let [mut s1, mut s2, mut s3] = [
        signal::signal(signal::SignalKind::hangup()).unwrap(),
        signal::signal(signal::SignalKind::interrupt()).unwrap(),
        signal::signal(signal::SignalKind::terminate()).unwrap(),
    ];

    tokio::select!(
        v = s1.recv() => v.unwrap(),
        v = s2.recv() => v.unwrap(),
        v = s3.recv() => v.unwrap(),
    );
}

#[cfg(windows)]
#[allow(clippy::unwrap_used)]
// The program is exiting so it doesn't matter if it panics
async fn wait_until_shutdown() {
    let (mut s1, mut s2) = (
        tokio::signal::windows::ctrl_c().unwrap(),
        tokio::signal::windows::ctrl_break().unwrap(),
    );

    tokio::select!(
        v = s1.recv() => v.unwrap(),
        v = s2.recv() => v.unwrap(),
    );
}
