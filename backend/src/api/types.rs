/// Types only for the api module
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub enum BeatmapStatus {
    Ranked,
    Qualified,
    Loved,
    Pending,
    Wip,
    Graveyard,
}

impl Display for BeatmapStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            BeatmapStatus::Ranked => write!(f, "Ranked"),
            BeatmapStatus::Qualified => write!(f, "Qualified"),
            BeatmapStatus::Loved => write!(f, "Loved"),
            BeatmapStatus::Pending => write!(f, "Disqualified"),
            BeatmapStatus::Wip => write!(f, "Disqualified"),
            BeatmapStatus::Graveyard => write!(f, "Disqualified"),
        }
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Modes {
    #[serde(rename = "osu")]
    Standard,
    #[serde(rename = "fruits")]
    Catch,
    Mania,
    Taiko,
}

impl Display for Modes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Modes::Standard => write!(f, "osu!standard"),
            Modes::Catch => write!(f, "osu!catch"),
            Modes::Mania => write!(f, "osu!mania"),
            Modes::Taiko => write!(f, "osu!taiko"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Nominators {
    pub user_id: i32,
}

#[derive(Deserialize, Debug)]
pub struct Beatmap {
    pub id: i32,
    #[serde(rename = "difficulty_rating")]
    pub star_rating: f32,
    pub mode: Modes,
    pub bpm: f32,
    #[serde(rename = "ranked")]
    pub ranked_status: BeatmapStatus,
}

#[derive(Deserialize, Debug)]
pub struct Beatmapset {
    pub id: i32,
    pub title: String,
    pub artist: String,
    #[serde(rename = "creator")]
    pub mapper: String,
    pub beatmaps: Vec<Beatmap>,
    #[serde(rename = "ranked")]
    pub ranked_status: BeatmapStatus,
    pub current_nominations: Vec<Nominators>,

    #[serde(rename = "ranked_date")]
    #[serde(deserialize_with = "deserialize_rfc3339_to_unix_timestamp")]
    pub ranked_date_unix: Option<i64>,

    #[serde(rename = "submitted_date")]
    #[serde(deserialize_with = "deserialize_rfc3339_to_unix_timestamp")]
    // NOTE this will always be Some(i64) because I don't want to bother with generics
    pub submitted_date_unix: Option<i64>,
}

fn deserialize_rfc3339_to_unix_timestamp<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_string: Option<String> = Option::deserialize(deserializer)?;

    if let Some(string) = opt_string {
        let datetime = DateTime::parse_from_rfc3339(&string)
            .map_err(serde::de::Error::custom)?
            .with_timezone(&Utc);
        Ok(Some(datetime.timestamp()))
    } else {
        Ok(None)
    }
}

impl<'de> Deserialize<'de> for BeatmapStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: i8 = Deserialize::deserialize(deserializer)?;
        Ok(match value {
            1 => BeatmapStatus::Ranked,
            3 => BeatmapStatus::Qualified,
            4 => BeatmapStatus::Loved,
            0 => BeatmapStatus::Pending,
            -1 => BeatmapStatus::Wip,
            -2 => BeatmapStatus::Graveyard,
            // Just assume its qualified rather than adding in unneeded code for linter
            _ => BeatmapStatus::Qualified,
        })
    }
}

#[derive(Deserialize)]
pub struct SearchResponse {
    #[serde(
        rename = "beatmapsets",
        deserialize_with = "deserialize_beatmapset_ids"
    )]
    pub beatmapset_ids: Vec<i32>,
    pub cursor_string: Option<String>,
}

fn deserialize_beatmapset_ids<'de, D>(deserializer: D) -> Result<Vec<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Beatmapset {
        id: i32,
    }

    let beatmapsets: Vec<Beatmapset> = Deserialize::deserialize(deserializer)?;
    Ok(beatmapsets.into_iter().map(|b| b.id).collect())
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Non HTTP 200 response from request, code: {0}")]
    Non200Response(i32),

    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
}
