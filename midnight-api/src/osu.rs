use anyhow::{Result, anyhow, bail};
use futures::{
    TryStreamExt,
    stream::{self, StreamExt},
};
use midnight_model::osu::{Beatmapset, BeatmapsetVec, SearchResponse};
use midnight_util::constants::{MAX_CONCURRENT_REQUESTS, OSU_BASE_URL, OSU_TOKEN_GRANT_URL};
use reqwest::{
    Client, RequestBuilder, Response, StatusCode,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use smallvec::SmallVec;
use tokio::sync::RwLock;
use tracing::{debug, warn};

const ACCESS_TOKEN_KEY: &str = r#""access_token":""#;

pub struct Osu {
    config: OsuConfig,
    client: Client,
    authorisation: RwLock<Option<String>>,
}

struct OsuConfig {
    client_id: u32,
    client_secret: Box<str>,
}

impl Osu {
    pub fn new(client_id: u32, client_secret: &str, reqwest_client: Client) -> Self {
        Self {
            config: OsuConfig {
                client_id,
                client_secret: client_secret.into(),
            },
            client: reqwest_client,
            authorisation: RwLock::new(None),
        }
    }

    async fn reauthorise(&self) -> Result<()> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let body = format!(
            "client_id={}&client_secret={}&grant_type=client_credentials&scope=public",
            self.config.client_id, self.config.client_secret
        );

        let res = self
            .client
            .post(OSU_TOKEN_GRANT_URL)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        let (code, text) = (res.status(), res.text().await?);

        match code {
            StatusCode::OK => {
                let start_idx = &text
                    .find(ACCESS_TOKEN_KEY)
                    .ok_or_else(|| anyhow!("Missing access token"))?;
                let content = start_idx + ACCESS_TOKEN_KEY.len();
                let end_idx = &text[content..]
                    .find(r#""}"#)
                    .ok_or_else(|| anyhow!("Missing access token"))?;

                let mut bearer = format!("Bearer {}", &text[content..content + end_idx].trim());
                bearer.shrink_to_fit();
                let mut guard = self.authorisation.write().await;
                *guard = Some(bearer)
            }
            StatusCode::UNAUTHORIZED => {
                bail!(
                    "Received 401 error while authorising with osu! API, \
                    make sure your environment variables are set correctly"
                )
            }
            _ => bail!("Status code: {code}, response: {text:?}"),
        }

        Ok(())
    }

    async fn request_with_auth(&self, builder: RequestBuilder) -> Result<Response> {
        let guard = self.authorisation.read().await;
        debug!("{builder:?}");
        match guard.as_ref() {
            Some(bearer) => {
                let mut res = builder
                    .try_clone()
                    .ok_or_else(|| anyhow!("Unable to clone builder"))?
                    .header(AUTHORIZATION, bearer.trim())
                    .send()
                    .await?;

                if res.status() == StatusCode::UNAUTHORIZED {
                    warn!("401 from osu! API, reauthorising");
                    self.reauthorise().await?;

                    let bearer = guard
                        .as_ref()
                        .expect("Bearer token should always be `Some` at this point");
                    res = builder.header(AUTHORIZATION, bearer.trim()).send().await?;
                }

                Ok(res)
            }
            None => {
                warn!("No bearer token, reauthorising");
                drop(guard);
                self.reauthorise().await?;
                let res = Box::pin(self.request_with_auth(builder)).await?;
                Ok(res)
            }
        }
    }

    pub async fn fetch_beatmaps(&self, ids: SmallVec<[i32; 8]>) -> Result<BeatmapsetVec> {
        stream::iter(ids.into_iter())
            .map(|id| self.fetch_beatmap(id))
            .buffer_unordered(MAX_CONCURRENT_REQUESTS)
            .try_collect()
            .await
    }

    async fn fetch_beatmap(&self, id: i32) -> Result<Beatmapset> {
        let url = format!("{}/beatmapsets/{}", OSU_BASE_URL, id);
        let res = self.request_with_auth(self.client.get(&url)).await?;

        let text = res.text().await?;
        Ok(serde_json::from_str::<Beatmapset>(&text)?)
    }

    pub async fn fetch_all_qualified_maps(&self) -> Result<Vec<i32>> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        // TODO maybe size hinting based off of average
        let mut ids: Vec<i32> = Vec::new();
        let mut cursor_string: Option<String> = Some("".to_string());

        loop {
            debug!(?cursor_string, "Running loop");
            match cursor_string {
                Some(cursor) => {
                    let res = self
                        .request_with_auth(
                            self.client
                                .get(format!(
                                    "{}/beatmapsets/search?nsfw=true&s=qualified&cursor_string={}",
                                    OSU_BASE_URL, cursor,
                                ))
                                .headers(headers.clone()),
                        )
                        .await?;

                    if !res.status().is_success() {
                        return Err(anyhow!("Non-success status code: {}", res.status()));
                    }

                    let text = res.text().await?;
                    let mut deserialized: SearchResponse = serde_json::from_str(&text)?;

                    cursor_string = deserialized.cursor_string;
                    debug!("Updated cursor sting, {:?}", cursor_string);
                    ids.append(&mut deserialized.beatmapset_ids);
                }
                None => break,
            }
        }

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;
    use std::env;

    #[tokio::test]
    async fn test_anything_and_everything() -> Result<()> {
        dotenv().ok();

        let client_id = env::var("OSU_API_CLIENT_ID")?;
        let client_secret = env::var("OSU_API_SECRET")?;
        let reqwest_client = Client::new();

        let osu = Osu::new(client_id.parse()?, &client_secret, reqwest_client);

        //osu.reauthorise().await?;
        println!("{:#?}", osu.fetch_all_qualified_maps().await?);

        Ok(())
    }
}
