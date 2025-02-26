pub use crate::database::OsuGamemode;
use crate::database::OsuGroup;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use smallvec::SmallVec;
use std::fmt::Display;

pub type BeatmapsetVec = SmallVec<[Beatmapset; 8]>;
type MemberOfDeserializeInner = Vec<(OsuGroup, SmallVec<[OsuGamemode; 4]>)>;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Deserialize)]
pub struct OsuUser {
    pub id: i32,
    pub username: String,
    pub avatar_url: String,
    #[serde(rename = "groups", deserialize_with = "deserialize_member_of")]
    pub member_of: Vec<(OsuGroup, SmallVec<[OsuGamemode; 4]>)>,
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
    // NOTE this will always be Some(i64) because I don't want to bother with a custom deserialization
    pub submitted_date_unix: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct Beatmap {
    pub id: i32,
    #[serde(rename = "difficulty_rating")]
    pub star_rating: f32,
    pub mode: OsuGamemode,
    pub bpm: f32,
    #[serde(rename = "ranked")]
    pub ranked_status: BeatmapStatus,
}

#[derive(Deserialize, Debug)]
pub struct Nominators {
    pub user_id: i32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GamemodeUpdate {
    pub group: OsuGroup,
    pub added: SmallVec<[OsuGamemode; 4]>,
    pub removed: SmallVec<[OsuGamemode; 4]>,
}

#[derive(Debug)]
pub struct Diff {
    pub added: Vec<OsuUser>,
    pub removed: Vec<OsuUser>,
    pub updated: Vec<(OsuUser, GamemodeUpdate)>,
}

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

fn deserialize_member_of<'de, D>(deserializer: D) -> Result<MemberOfDeserializeInner, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize, Debug)]
    struct Group {
        identifier: OsuGroup,
        playmodes: Option<SmallVec<[OsuGamemode; 4]>>,
    }

    let member_of: Vec<Group> = Vec::deserialize(deserializer)?;
    Ok(member_of
        .into_iter()
        .map(|group| (group.identifier, group.playmodes.unwrap_or_default()))
        .collect())
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

fn deserialize_beatmapset_ids<'de, D>(deserializer: D) -> Result<Vec<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Beatmapset {
        id: i32,
    }

    let beatmapsets: Vec<Beatmapset> = Deserialize::deserialize(deserializer)?;
    Ok(beatmapsets.into_iter().map(|b| b.id).collect())
}
