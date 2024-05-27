use fancy_regex::Regex;
use reqwest::Client;
use std::sync::Arc;

lazy_static! {
    pub static ref CLIENT: Arc<Client> = Arc::new(Client::new());
    pub static ref TWITTER_REGEX: Regex = Regex::new(r".*twitter.*").unwrap();
}

pub mod safebooru;
