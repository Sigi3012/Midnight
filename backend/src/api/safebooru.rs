use crate::{REQWEST_CLIENT, api::TWITTER_REGEX};
use log::{debug, info};
use rand::{Rng, thread_rng};
use serde::Deserialize;
use thiserror::Error;

type Result<T> = std::result::Result<T, SafebooruFetchError>;

#[derive(Error, Debug)]
pub enum SafebooruFetchError {
    #[error("Request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Failed to parse XML: {0}")]
    Xml(#[from] serde_xml_rs::Error),
}

#[derive(Debug, Deserialize)]
struct Post {
    file_url: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct Posts {
    #[serde(rename = "post", default)]
    posts: Vec<Post>,
}

// Page range for the tags provided, as of 27.05.2024
const PAGE_RANGE: std::ops::Range<i16> = 1..148;
const BASE_URL: &str = "https://safebooru.org/index.php?page=dapi&s=post&q=index&pid={page_id}&limit=100&tags=yuri+2girls";

// Because thread_rng uses well, threads, we have this function to call .await on
async fn generate_number(range: std::ops::Range<i16>) -> i16 {
    let mut rng = thread_rng();
    rng.gen_range(range)
}

pub async fn get_random_post(count: i32) -> Result<Vec<(String, Option<String>)>> {
    let mut links: Vec<(String, Option<String>)> = Vec::new();

    info!("Attempting to fetch {} posts", count);

    let page_number = generate_number(PAGE_RANGE).await.to_string();
    let url = BASE_URL.replace("{page_id}", &page_number);

    let response = REQWEST_CLIENT.get(&url).send().await?;
    let xml = response.text().await?;
    let p: Posts = serde_xml_rs::from_str(&xml)?;

    for _ in 0..count {
        let random_post: usize = generate_number(1..100).await as usize;
        let post_data: &Post = &p.posts[random_post];

        if TWITTER_REGEX.is_match(&post_data.source).unwrap_or(false) {
            links.push((post_data.file_url.clone(), Some(post_data.source.clone())));
        } else {
            links.push((post_data.file_url.clone(), None));
        }
    }

    info!("Successfully fetched posts");
    debug!("{:?}", links);

    Ok(links)
}
