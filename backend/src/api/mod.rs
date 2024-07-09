use fancy_regex::Regex;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static! {
    pub static ref CLIENT: Arc<Client> = Arc::new(Client::new());
    pub static ref TWITTER_REGEX: Regex = Regex::new(r".*twitter.*").unwrap();
    pub static ref CLIENT_ID: String =
        std::env::var("OSU_API_CLIENT_ID").expect("Please set osu! api client id in env");
    pub static ref API_SECRET: String =
        std::env::var("OSU_API_SECRET").expect("Please set osu! api secret key in env");
    pub static ref ACCESS_TOKEN: Mutex<Option<String>> = Mutex::new(None);
}

pub mod types;

pub mod osu;
pub mod safebooru;
