use common::context::set_context_wrapper;
use database::core::initialize_database;
use dotenv;
use log::{error, info, warn};
use poise::serenity_prelude as serenity;
use std::process::exit;

mod commands;
mod events;
mod tasks;

pub struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let token = std::env::var("DISCORD_TOKEN").unwrap_or_else(|err| {
        error!("Missing DISCORD_TOKEN: {}", err);
        exit(1);
    });

    info!("Logger and enviroment variables initalised");

    match initialize_database().await {
        Ok(_) => info!("Database initalized"),
        Err(e) => {
            error!(
                "Fatal! Something went wrong initalizing the database, {}",
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
            commands::cat::cat(),
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

                Ok(Data {})
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

        warn!("Recieved Ctrl+C, closing shards..");
        shard_manager.shutdown_all().await;
        info!("All shards closed");
    });

    info!("Starting bot");

    if let Err(err) = client.start_autosharded().await {
        error!("Client error: {}", err)
    }
}

#[cfg(unix)]
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
