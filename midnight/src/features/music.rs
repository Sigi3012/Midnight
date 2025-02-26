use crate::context::Context;
use fancy_regex::Regex;
use midnight_database::subscriptions::{
    ChannelType, SubscriptionMode, fetch_all_subscribed_channels,
};
use poise::serenity_prelude::all::Message;
use std::{
    collections::HashSet,
    io::{self, Read},
    process::{Command, Stdio},
    sync::LazyLock,
};
use thiserror::Error;
use tokio::{
    sync::{Mutex, OnceCell},
    task::{self, JoinError},
    time::{self, Duration, error::Elapsed},
};
use tracing::{error, info, warn};

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(7);

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https?:\/\/(?:youtu\.be|(?:www\.|music\.)?youtube\.com)\/(?:watch\?v=)?([\w-]+)"#)
        .expect("Regex should compile")
});
pub(crate) static CHANNEL_CACHE: LazyLock<ChannelCache> = LazyLock::new(ChannelCache::new);

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("{0}")]
    IoError(#[from] io::Error),

    #[error("{0}")]
    JoinError(#[from] JoinError),

    #[error("")]
    FileTooLarge,

    #[error("Download took too long, {0}")]
    DownloadTimeout(#[from] Elapsed),
}

pub struct ChannelCache {
    initialized: OnceCell<()>,
    channels: Mutex<HashSet<i64>>,
}

pub struct Song(Vec<u8>);

impl ChannelCache {
    pub fn new() -> Self {
        Self {
            initialized: OnceCell::new(),
            channels: Mutex::new(HashSet::new()),
        }
    }

    pub async fn check(&self, id: i64) -> bool {
        self.ensure_initialized().await;
        let guard = self.channels.lock().await;
        guard.contains(&id)
    }

    pub async fn update_cache(&self) {
        if self.ensure_initialized().await {
            let mut conn = Context::database().get_conn().await;
            match fetch_all_subscribed_channels(
                &mut conn,
                ChannelType::Music(SubscriptionMode::Subscribe),
            )
            .await
            {
                Ok(ids) => {
                    let stored: HashSet<i64> = ids.into_iter().collect();

                    let mut guard = self.channels.lock().await;
                    if stored.is_empty() {
                        guard.clear();
                        return;
                    }

                    let new_ids: HashSet<i64> = stored.difference(&guard).cloned().collect();
                    let removed_ids: HashSet<i64> = guard.difference(&stored).cloned().collect();

                    new_ids.iter().for_each(|i| {
                        guard.insert(*i);
                    });
                    removed_ids.iter().for_each(|i| {
                        guard.remove(i);
                    });
                    info!("Updated music channel cache successfully")
                }
                Err(e) => {
                    error!("Failed to update music channel cache, {}", e)
                }
            }
        }
    }

    async fn populate(&self) {
        let mut guard = self.channels.lock().await;
        let mut conn = Context::database().get_conn().await;
        if let Ok(ids) = fetch_all_subscribed_channels(
            &mut conn,
            ChannelType::Music(SubscriptionMode::Subscribe),
        )
        .await
        {
            *guard = ids.iter().cloned().collect::<HashSet<i64>>()
        }

        info!("Music channel cache initialized");
    }

    async fn ensure_initialized(&self) -> bool {
        if self.initialized.get().is_none() {
            self.populate().await;
            self.initialized.set(()).ok();
            false
        } else {
            true
        }
    }
}

impl Song {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn get(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub async fn music_link_handler(message: &Message) -> Result<Option<Song>, DownloadError> {
    if let Some(url) = check_link(&message.content) {
        if CHANNEL_CACHE.check(message.channel_id.get() as i64).await {
            return match download_audio(url.clone()).await {
                Ok(song) => {
                    if song.len() > MAX_FILE_SIZE {
                        Err(DownloadError::FileTooLarge)
                    } else {
                        Ok(Some(song))
                    }
                }
                Err(e) => {
                    error!("Failed to download, link: {}, error: {}", url, e);
                    Err(e)
                }
            };
        }
    }

    Ok(None)
}

fn check_link(message: &str) -> Option<String> {
    match REGEX.captures(message) {
        Ok(Some(captures)) => captures.get(1).map(|content| content.as_str().to_owned()),
        Ok(None) => None,
        Err(e) => {
            warn!("Failed to get captures on {}, error: {}", message, e);
            None
        }
    }
}

async fn download_audio(id: String) -> Result<Song, DownloadError> {
    info!("Fetching audio for {}", &id);

    let audio_data = time::timeout(
        DOWNLOAD_TIMEOUT,
        task::spawn_blocking(move || -> Result<Vec<u8>, DownloadError> {
            let mut yt_dlp = Command::new("yt-dlp")
                .args(["-o", "-", "-x", &id])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?;

            let mut output = Vec::new();
            if let Some(ref mut stdout) = yt_dlp.stdout {
                stdout.read_to_end(&mut output)?;
            }

            yt_dlp.wait()?;
            Ok(output)
        }),
    )
    .await???;

    Ok(Song::new(audio_data))
}

#[cfg(test)]
mod tests {
    // Rust analyzer is stupid
    #![allow(unused_imports, clippy::unwrap_used)]
    use super::*;

    macro_rules! regex_test {
        ($name:ident, $input:expr, $expected_capture:expr) => {
            #[test]
            fn $name() {
                assert!(REGEX.is_match($input).unwrap());
                assert_eq!(
                    REGEX
                        .captures($input)
                        .unwrap()
                        .unwrap()
                        .get(1)
                        .unwrap()
                        .as_str(),
                    $expected_capture
                );
            }
        };
    }

    regex_test!(
        test_www,
        "https://www.youtube.com/watch?v=HOz-9FzIDf0",
        "HOz-9FzIDf0"
    );

    regex_test!(
        test_music,
        "https://music.youtube.com/watch?v=lUQjaC5IaMA&si=uATM_kEIlpWDfwOI",
        "lUQjaC5IaMA"
    );

    regex_test!(
        test_shortened,
        "https://youtu.be/xCMqBDWr-bk?si=BnST6uCCjEZ7uJpN",
        "xCMqBDWr-bk"
    );
}
