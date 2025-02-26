use crate::config::BotConfiguration;
use anyhow::{Result, bail};
use midnight_api::{Fun, Osu};
use midnight_database::core::Database;
use poise::serenity_prelude::all::{Cache, CacheHttp, Http, ShardMessenger};
use std::sync::{Arc, OnceLock};
use tokio::time::Instant;

static CONTEXT: OnceLock<Box<Context>> = OnceLock::new();

pub struct Context {
    clients: Clients,
    discord_ctx: DiscordContextWrapper,
    startup_time: Instant,
}

struct Clients {
    http: reqwest::Client,
    osu: Osu,
    fun: Fun,
    db: Database,
}

pub struct DiscordContextWrapper {
    pub shard: ShardMessenger,
    pub cache: Arc<Cache>,
    pub http: Arc<Http>,
}

impl Context {
    #[allow(clippy::new_ret_no_self)]
    pub async fn new(config: &BotConfiguration, discord_ctx: DiscordContextWrapper) -> Result<()> {
        let reqwest_client = reqwest::Client::new();
        let osu = Osu::new(
            config.tokens.osu_client_id,
            config.tokens.osu_secret.as_ref(),
            reqwest_client.clone(),
        );
        let fun = Fun::new(reqwest_client.clone(), &config.tokens.thecatapi_secret);
        let db = match Database::new(&config.database_url).await {
            Ok(db) => db,
            Err(e) => bail!("Failed to initialise database, {e}"),
        };

        let clients = Clients {
            http: reqwest_client,
            osu,
            db,
            fun,
        };

        let ctx = Self {
            clients,
            discord_ctx,
            startup_time: Instant::now(),
        };

        if CONTEXT.set(Box::new(ctx)).is_err() {
            panic!("must init Context only once");
        }

        Ok(())
    }

    fn get() -> &'static Self {
        CONTEXT.get().expect("Context should be initialised")
    }

    pub fn http() -> &'static reqwest::Client {
        &Self::get().clients.http
    }
    pub fn osu() -> &'static Osu {
        &Self::get().clients.osu
    }
    pub fn database() -> &'static Database {
        &Self::get().clients.db
    }
    pub fn fun() -> &'static Fun {
        &Self::get().clients.fun
    }
    pub fn discord_ctx() -> &'static DiscordContextWrapper {
        &Self::get().discord_ctx
    }
    pub fn startup_time() -> &'static Instant {
        &Self::get().startup_time
    }
}

impl CacheHttp for DiscordContextWrapper {
    fn http(&self) -> &Http {
        &self.http
    }
    fn cache(&self) -> Option<&Arc<Cache>> {
        Some(&self.cache)
    }
}
