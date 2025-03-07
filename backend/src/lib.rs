#![feature(duration_constructors)]

use reqwest::Client;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref REQWEST_CLIENT: Client = Client::new();
}

pub mod api;
pub mod groups;
pub mod links;
pub mod mapfeed;
pub mod music;
pub mod sticky;
