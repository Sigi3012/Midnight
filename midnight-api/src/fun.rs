use anyhow::{Context, Result};
use fancy_regex::Regex;
use midnight_model::cat::ResponseJson;
use midnight_util::constants::{BOORU_PAGE_RANGE, SAFEBOORU_BASE_URL, THECATAPI_BASE_URL};
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderName};
use serde::Deserialize;
use std::sync::LazyLock;
use tracing::log::{debug, info};

static TWITTER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".*twitter.*").expect("Regex should compile"));

pub struct Fun {
    client: reqwest::Client,
    // Create a config struct if there is more api keys added
    thecatapi_secret: Box<str>,
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

impl Fun {
    pub fn new(reqwest_client: reqwest::Client, thecatapi_secret: &str) -> Self {
        Self {
            client: reqwest_client,
            thecatapi_secret: thecatapi_secret.into(),
        }
    }

    pub async fn get_random_cat_image(&self, count: i32) -> Result<Vec<ResponseJson>> {
        info!("Attempting to fetch {} images", count);

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-api-key"),
            self.thecatapi_secret
                .parse()
                .context("Failed to parse api token")?,
        );

        let mut responses = Vec::new();
        responses.reserve_exact(count as usize);

        for _ in 0..count {
            let response = self
                .client
                .get(THECATAPI_BASE_URL)
                .headers(headers.clone())
                .send()
                .await?;

            responses.push(response.text().await?);
        }

        let result = ResponseJson::from_strings(responses)?;
        Ok(result)
    }

    pub async fn get_random_booru_post(&self, count: i32) -> Result<Vec<(String, Option<String>)>> {
        let mut links: Vec<(String, Option<String>)> = Vec::new();

        info!("Attempting to fetch {} posts", count);

        let page_number = generate_number(BOORU_PAGE_RANGE).await.to_string();
        let mut url = SAFEBOORU_BASE_URL.replace("{page_id}", &page_number);
        url.push_str("&tags=yuri+2girls");

        let response = self.client.get(&url).send().await?;
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
}

// Because thread_rng uses well, threads, we have this function to call .await on
async fn generate_number(range: std::ops::Range<i16>) -> i16 {
    let mut rng = rand::rng();
    rng.random_range(range)
}
