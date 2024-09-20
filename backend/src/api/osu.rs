use crate::{
    api::{
        types::{Beatmapset, SearchResponse},
        {ACCESS_TOKEN, OSU_API_SECRET, OSU_CLIENT_ID},
    },
    REQWEST_CLIENT,
};
use log::{debug, error, info, warn};
use reqwest::header::{HeaderMap, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tokio::{
    task,
    time::{sleep, Duration},
};

const REQUEST_THREAD_COUNT: usize = 16;

const BASE_API_URL: &str = "https://osu.ppy.sh/api/v2";
const GRANT_URL: &str = "https://osu.ppy.sh/oauth/token";

#[derive(Deserialize, Debug, Clone)]
pub struct AuthenticationManager {
    access_token: String,
    expires_in: u64,
}

impl AuthenticationManager {
    #[allow(clippy::new_ret_no_self)]
    pub async fn new() {
        task::spawn(Self::refresh_token());
    }

    async fn authenticate() -> Result<AuthenticationManager, Box<dyn std::error::Error>> {
        let client = REQWEST_CLIENT.clone();

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
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
            let text = res.text().await.unwrap();
            let deserialized: AuthenticationManager = serde_json::from_str(&text).unwrap();

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

pub async fn fetch_all_qualified_maps() -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let client = REQWEST_CLIENT.clone();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());

    if let Some(ref token) = *ACCESS_TOKEN.lock().await {
        let formatted = format!("Bearer {}", token);

        headers.insert(AUTHORIZATION, formatted.parse().unwrap());
    };

    let mut ids: Vec<i32> = Vec::new();
    let mut cursor_string: Option<String> = Some("".to_string());

    loop {
        debug!("Running loop, {:?}", cursor_string);
        if cursor_string.is_none() {
            break;
        }

        let res = client
            .get(format!(
                "{}/beatmapsets/search?nsfw=true&s=qualified&cursor_string={}",
                BASE_API_URL,
                cursor_string.unwrap()
            ))
            .headers(headers.clone())
            .send()
            .await?;

        if res.status().is_success() {
            let text = res.text().await.unwrap();
            let mut deserialized: SearchResponse = serde_json::from_str(&text).unwrap();

            cursor_string = deserialized.cursor_string;
            debug!("Update cursor sting, {:?}", cursor_string);
            ids.append(&mut deserialized.beatmapset_ids);
        } else {
            return Err(format!("Non-success status code: {}", res.status()).into());
        };
    }

    Ok(ids)
}

pub async fn fetch_beatmaps(ids: Vec<i32>) -> Result<Vec<Beatmapset>, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();

    if let Some(ref token) = *ACCESS_TOKEN.lock().await {
        let formatted = format!("Bearer {}", token);

        headers.insert(AUTHORIZATION, formatted.parse()?);
    };

    let responses: Arc<Mutex<Vec<Beatmapset>>> = Arc::new(Mutex::new(Vec::new()));

    let client = REQWEST_CLIENT.clone();
    let semaphore = Arc::new(Semaphore::new(REQUEST_THREAD_COUNT));
    let mut handles = Vec::new();

    // Stupid implementation
    let loop_ids = ids.clone();
    for id in loop_ids {
        let client = client.clone();
        let headers = headers.clone();
        let semaphore = Arc::clone(&semaphore);
        let responses = Arc::clone(&responses);
        let url = format!("{}/beatmapsets/{}", BASE_API_URL, id);

        let permit = semaphore.acquire_owned().await?;

        let handle = tokio::spawn(async move {
            let response = client.get(&url).headers(headers).send().await;

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        let text = res.text().await.unwrap();
                        debug!("Success: ID {}", id);
                        match serde_json::from_str(&text) {
                            Ok(deserialized) => {
                                let mut responses = responses.lock().unwrap();
                                responses.push(deserialized)
                            }
                            Err(e) => {
                                error!("An error occurred while deserializing json: {}", e);
                            }
                        };
                    } else {
                        warn!("Failed: ID {} - Status: {:?}", id, res.status());
                    }
                }
                Err(e) => {
                    warn!("Error: ID {} - Error: {:?}", id, e);
                }
            }

            drop(permit);
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    let mut response_guard = responses.lock().unwrap();
    let response_vec = std::mem::take(&mut *response_guard);

    if response_vec.len() != ids.len() {
        error!("An unexpected amount of responses were returned")
        // TODO return custom Err
    }

    Ok(response_vec)
}
