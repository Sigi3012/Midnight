use std::{env, sync::LazyLock};

pub static CONFIG: LazyLock<BotConfiguration> = LazyLock::new(BotConfiguration::new);

pub struct BotConfiguration {
    pub tokens: Tokens,
    pub database_url: Box<str>,
}

pub struct Tokens {
    pub discord: Box<str>,
    pub osu_client_id: u32,
    pub osu_secret: Box<str>,
    pub thecatapi_secret: Box<str>,
}

impl BotConfiguration {
    fn new() -> Self {
        Self {
            tokens: Tokens::new(),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL should be set .env")
                .into_boxed_str(),
        }
    }
}

impl Tokens {
    fn new() -> Self {
        Self {
            discord: env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN should be set in .env")
                .into_boxed_str(),
            osu_client_id: env::var("OSU_API_CLIENT_ID")
                .expect("OSU_API_CLIENT_ID should be set in .env")
                .parse::<u32>()
                .expect("OSU_API_CLIENT_ID should be an integer"),
            osu_secret: env::var("OSU_API_SECRET")
                .expect("OSU_API_SECRET should be set in .env")
                .into_boxed_str(),
            thecatapi_secret: env::var("CAT_API_SECRET")
                .expect("CAT_API_SECRET should be set in .env")
                .into_boxed_str(),
        }
    }
}
