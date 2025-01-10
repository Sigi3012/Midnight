use common::context::set_context_wrapper;
use poise::serenity_prelude as serenity;
use std::process::exit;
use tokio::time::Instant;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod events;
mod tasks;

pub struct Data {
    startup_time: Instant,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "bot=debug,backend=debug,database=debug,serenity=warn,poise=warn".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let token = std::env::var("DISCORD_TOKEN").unwrap_or_else(|err| {
        error!("Missing DISCORD_TOKEN: {}", err);
        exit(1);
    });

    info!("Logger and environment variables initalised");

    match database::core::initialise().await {
        Ok(_) => info!("Database initialised"),
        Err(e) => {
            error!(
                "Fatal! Something went wrong initialising the database, {}",
                e
            );
            return;
        }
    }

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
                tasks::init_tasks().await;
                set_context_wrapper(ctx.shard.clone(), ctx.http.clone(), ctx.cache.clone());

                Ok(Data {
                    startup_time: Instant::now(),
                })
            })
        })
        .build();

    let mut client: serenity::Client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .unwrap_or_else(|err| {
            error!("Error while creating client: {}", err);
            exit(1);
        });

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
