use crate::{
    api::{
        types::{Beatmapset, SearchResponse},
        {ACCESS_TOKEN, OSU_API_SECRET, OSU_CLIENT_ID},
    },
    REQWEST_CLIENT,
};
use anyhow::{anyhow, Result};
use futures::stream::{self, StreamExt};
use log::{debug, info};
use reqwest::header::{HeaderMap, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use smallvec::SmallVec;
use tokio::{
    task,
    time::{sleep, Duration},
};

pub type BeatmapsetVec = SmallVec<[Beatmapset; 8]>;

const MAX_CONCURENT_REQUESTS: usize = 16;

const BASE_API_URL: &str = "https://osu.ppy.sh/api/v2";
const GRANT_URL: &str = "https://osu.ppy.sh/oauth/token";

#[derive(Deserialize, Debug, Clone)]
pub struct AuthenticationManager {
    access_token: String,
    expires_in: u64,
}

// TODO
// Lazy load the access token
// Convert new to non-async and change `access_token` to Option<String>
// Remove `refresh_token()`
impl AuthenticationManager {
    #[allow(clippy::new_ret_no_self)]
    pub async fn new() {
        task::spawn(Self::refresh_token());
    }

    async fn authenticate() -> Result<AuthenticationManager, Box<dyn std::error::Error>> {
        let client = REQWEST_CLIENT.clone();

        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            "application/json".parse().expect("Header should parse"),
        );
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded"
                .parse()
                .expect("Header should parse"),
        );

        let body = format!(
            "client_id={}&client_secret={}&grant_type=client_credentials&scope=public",
            *OSU_CLIENT_ID, *OSU_API_SECRET
        );

        let res = client
            .post(GRANT_URL)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        if res.status().is_success() {
            let text = res.text().await?;
            let deserialized: AuthenticationManager = serde_json::from_str(&text)?;

            Ok(deserialized)
        } else {
            Err(format!("Non-success status code: {}", res.status()).into())
        }
    }

    async fn refresh_token() {
        loop {
            let token = Self::authenticate().await.expect("Failed to fetch token");
            {
                let mut token_lock = ACCESS_TOKEN.lock().await;
                *token_lock = Some(token.access_token.clone());
            }

            info!("Successfully authenticated");

            // Refresh 1 minute before expiry
            let sleep_duration = token.expires_in - 60;
            sleep(Duration::from_secs(sleep_duration)).await;
        }
    }
}

pub async fn fetch_all_qualified_maps() -> Result<Vec<i32>> {
    let client = REQWEST_CLIENT.clone();

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        "application/json".parse().expect("Header should parse"),
    );
    headers.insert(
        ACCEPT,
        "application/json".parse().expect("Header should parse"),
    );

    if let Some(ref token) = *ACCESS_TOKEN.lock().await {
        let formatted = format!("Bearer {}", token);

        headers.insert(
            AUTHORIZATION,
            formatted.parse().expect("Access token header should parse"),
        );
    };

    let mut ids: Vec<i32> = Vec::new();
    let mut cursor_string: Option<String> = Some("".to_string());

    loop {
        debug!("Running loop, {:?}", cursor_string);
        match cursor_string {
            Some(cursor) => {
                let res = client
                    .get(format!(
                        "{}/beatmapsets/search?nsfw=true&s=qualified&cursor_string={}",
                        BASE_API_URL, cursor,
                    ))
                    .headers(headers.clone())
                    .send()
                    .await?;

                if res.status().is_success() {
                    let text = res.text().await?;
                    let mut deserialized: SearchResponse = serde_json::from_str(&text)?;

                    cursor_string = deserialized.cursor_string;
                    debug!("Update cursor sting, {:?}", cursor_string);
                    ids.append(&mut deserialized.beatmapset_ids);
                } else {
                    return Err(anyhow!("Non-success status code: {}", res.status()));
                };
            }
            None => break,
        }
    }

    Ok(ids)
}

pub async fn fetch_beatmaps(ids: Vec<i32>) -> Result<BeatmapsetVec> {
    let headers = build_headers().await?;
    let client = REQWEST_CLIENT.clone();

    let beatmaps: BeatmapsetVec = stream::iter(ids.into_iter())
        .map(|id| fetch_beatmap(id, &client, headers.clone()))
        .buffer_unordered(MAX_CONCURENT_REQUESTS)
        .filter_map(|result| futures::future::ready(result.ok()))
        .collect()
        .await;

    Ok(beatmaps)
}

async fn build_headers() -> Result<HeaderMap, anyhow::Error> {
    ACCESS_TOKEN
        .lock()
        .await
        .as_ref()
        .map(|token| {
            let mut headers = HeaderMap::with_capacity(1);
            #[allow(clippy::unwrap_used)]
            headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());
            headers
        })
        .ok_or_else(|| anyhow!("ACCESS_TOKEN is missing"))
}

async fn fetch_beatmap(
    id: i32,
    client: &reqwest::Client,
    headers: HeaderMap,
) -> Result<Beatmapset> {
    let url = format!("{}/beatmapsets/{}", BASE_API_URL, id);

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .and_then(|res| res.error_for_status())?;

    let text = response.text().await?;
    Ok(serde_json::from_str::<Beatmapset>(&text)?)
}
