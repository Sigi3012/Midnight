use fancy_regex::Regex;
use tokio::sync::Mutex;

lazy_static! {
    pub static ref TWITTER_REGEX: Regex = Regex::new(r".*twitter.*").expect("Regex should compile");
    pub static ref OSU_CLIENT_ID: String = std::env::var("OSU_API_CLIENT_ID")
        .expect("OSU_API_CLIENT_ID should be set in .env or .docker-compose.yml");
    pub static ref OSU_API_SECRET: String = std::env::var("OSU_API_SECRET")
        .expect("OSU_API_SECRET should be set in .env or .docker-compose.yml");
    pub static ref CAT_API_SECRET: String = std::env::var("CAT_API_SECRET")
        .expect("CAT_API_SECRET should be set in .env or .docker-compose.yml");
    pub static ref ACCESS_TOKEN: Mutex<Option<String>> = Mutex::new(None);
}

pub mod cat;
pub mod osu;
pub mod safebooru;
pub mod types;
