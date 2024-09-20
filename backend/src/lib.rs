use reqwest::Client;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref REQWEST_CLIENT: Client = Client::new();
}

pub mod types;

pub mod api;
pub mod links;
pub mod mapfeed;
