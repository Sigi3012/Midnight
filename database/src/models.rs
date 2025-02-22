use crate::schema::{
    beatmapset_subscriptions, beatmapsets, osu_user_group_gamemodes, osu_user_groups, osu_users,
    sticky_messages, subscriptions,
};
use diesel::{
    AsExpression, Associations, FromSqlRow, Identifiable, Insertable, Queryable, Selectable,
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    serialize::{IsNull, Output, ToSql},
};
use serde::Deserialize;
use std::{
    fmt::{Display, Formatter},
    io::Write,
};

#[derive(Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::ChannelKind)]
pub enum ChannelKind {
    Mapfeed,
    Music,
    Groups,
}

impl ToSql<crate::schema::sql_types::ChannelKind, Pg> for ChannelKind {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match *self {
            ChannelKind::Mapfeed => out.write_all(b"mapfeed")?,
            ChannelKind::Music => out.write_all(b"music")?,
            ChannelKind::Groups => out.write_all(b"groups")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::ChannelKind, Pg> for ChannelKind {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"mapfeed" => Ok(ChannelKind::Mapfeed),
            b"music" => Ok(ChannelKind::Music),
            b"groups" => Ok(ChannelKind::Groups),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

// TODO
// Remove the "Alumni" group and create a `NonTracked` enum variant to future proof
// any future groups being added to the osu api
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, Deserialize, AsExpression, FromSqlRow)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
#[diesel(sql_type = crate::schema::sql_types::OsuGroup)]
pub enum OsuGroup {
    #[serde(rename = "bng")]
    BeatmapNominator,
    #[serde(rename = "bng_limited")]
    ProbationaryBeatmapNominator,
    #[serde(rename = "nat")]
    NominationAssessmentTeam,
    #[serde(rename = "tc")]
    TournamentCommittee,
    #[serde(rename = "gmt")]
    GlobalModerationTeam,
    #[serde(rename = "dev")]
    Developer,
    #[serde(rename = "featured_artist")]
    FeatureArtist,
    #[serde(rename = "bsc")]
    BeatmapSpotlightCurator,
    #[serde(rename = "loved")]
    ProjectLoved,
    #[serde(rename = "support")]
    TechnicalSupportTeam,
    Ppy,
    Bot,
    Alumni,
}

impl ToSql<crate::schema::sql_types::OsuGroup, Pg> for OsuGroup {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match *self {
            OsuGroup::BeatmapNominator => out.write_all(b"BeatmapNominator")?,
            OsuGroup::ProbationaryBeatmapNominator => {
                out.write_all(b"ProbationaryBeatmapNominator")?
            }
            OsuGroup::NominationAssessmentTeam => out.write_all(b"NominationAssessmentTeam")?,
            OsuGroup::GlobalModerationTeam => out.write_all(b"GlobalModerationTeam")?,
            OsuGroup::TournamentCommittee => out.write_all(b"TournamentCommittee")?,
            OsuGroup::Developer => out.write_all(b"Developer")?,
            OsuGroup::FeatureArtist => out.write_all(b"FeatureArtist")?,
            OsuGroup::BeatmapSpotlightCurator => out.write_all(b"BeatmapSpotlightCurator")?,
            OsuGroup::ProjectLoved => out.write_all(b"ProjectLoved")?,
            OsuGroup::TechnicalSupportTeam => out.write_all(b"TechnicalSupportTeam")?,
            OsuGroup::Ppy => out.write_all(b"Ppy")?,
            OsuGroup::Bot => out.write_all(b"Bot")?,
            OsuGroup::Alumni => out.write_all(b"Alumni")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::OsuGroup, Pg> for OsuGroup {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"BeatmapNominator" => Ok(OsuGroup::BeatmapNominator),
            b"ProbationaryBeatmapNominator" => Ok(OsuGroup::ProbationaryBeatmapNominator),
            b"NominationAssessmentTeam" => Ok(OsuGroup::NominationAssessmentTeam),
            b"GlobalModerationTeam" => Ok(OsuGroup::GlobalModerationTeam),
            b"TournamentCommittee" => Ok(OsuGroup::TournamentCommittee),
            b"Developer" => Ok(OsuGroup::Developer),
            b"FeatureArtist" => Ok(OsuGroup::FeatureArtist),
            b"BeatmapSpotlightCurator" => Ok(OsuGroup::BeatmapSpotlightCurator),
            b"ProjectLoved" => Ok(OsuGroup::ProjectLoved),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl Display for OsuGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            OsuGroup::BeatmapNominator => write!(f, "Beatmap Nominators"),
            OsuGroup::ProbationaryBeatmapNominator => write!(f, "Probationary Beatmap Nominators"),
            OsuGroup::NominationAssessmentTeam => write!(f, "Nomination Assessment Team"),
            OsuGroup::GlobalModerationTeam => write!(f, "Global Moderation Team"),
            OsuGroup::TournamentCommittee => write!(f, "Tournament Committee"),
            OsuGroup::Developer => write!(f, "Developers"),
            OsuGroup::FeatureArtist => write!(f, "Feature Artists"),
            OsuGroup::BeatmapSpotlightCurator => write!(f, "Beatmap Spotlight Curators"),
            OsuGroup::ProjectLoved => write!(f, "Project Loved"),
            OsuGroup::TechnicalSupportTeam => write!(f, "Technical Support Team"),
            OsuGroup::Ppy => write!(f, "ppy"),
            OsuGroup::Bot => write!(f, "Bot"),
            OsuGroup::Alumni => write!(f, "Alumni"),
        }
    }
}

impl OsuGroup {
    pub fn id(&self) -> u8 {
        match *self {
            OsuGroup::BeatmapNominator => 28,
            OsuGroup::ProbationaryBeatmapNominator => 32,
            OsuGroup::NominationAssessmentTeam => 7,
            OsuGroup::GlobalModerationTeam => 4,
            OsuGroup::TournamentCommittee => 50,
            OsuGroup::Developer => 11,
            OsuGroup::FeatureArtist => 35,
            OsuGroup::BeatmapSpotlightCurator => 48,
            OsuGroup::ProjectLoved => 31,
            OsuGroup::TechnicalSupportTeam => 22,
            // This guy...
            OsuGroup::Ppy => 33,
            // This isn't actually displayed on the website for some reason so scraping it will error but that's fine as it won't be tracked anyway
            OsuGroup::Bot => 29,
            OsuGroup::Alumni => 16,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, Deserialize, AsExpression, FromSqlRow)]
#[serde(rename_all = "lowercase")]
#[diesel(sql_type = crate::schema::sql_types::OsuGamemode)]
pub enum OsuGamemode {
    Osu,
    Mania,
    Taiko,
    Fruits,
}

impl ToSql<crate::schema::sql_types::OsuGamemode, Pg> for OsuGamemode {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match *self {
            OsuGamemode::Osu => out.write_all(b"Standard")?,
            OsuGamemode::Mania => out.write_all(b"Mania")?,
            OsuGamemode::Taiko => out.write_all(b"Taiko")?,
            OsuGamemode::Fruits => out.write_all(b"Fruits")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::OsuGamemode, Pg> for OsuGamemode {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Standard" => Ok(OsuGamemode::Osu),
            b"Mania" => Ok(OsuGamemode::Mania),
            b"Taiko" => Ok(OsuGamemode::Taiko),
            b"Fruits" => Ok(OsuGamemode::Fruits),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl Display for OsuGamemode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            OsuGamemode::Osu => write!(f, "osu!Standard"),
            OsuGamemode::Mania => write!(f, "osu!Mania"),
            OsuGamemode::Taiko => write!(f, "osu!Taiko"),
            OsuGamemode::Fruits => write!(f, "osu!Catch"),
        }
    }
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = beatmapsets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Beatmapsets {
    pub id: i32,
}

#[derive(Queryable, Selectable, Associations, Identifiable)]
#[diesel(table_name = beatmapset_subscriptions)]
#[diesel(belongs_to(Beatmapsets, foreign_key = beatmapset_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BeatmapsetSubscriptions {
    pub id: i32,
    pub user_id: i64,
    pub beatmapset_id: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = subscriptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Subscriptions {
    pub channel_id: i64,
    pub kind: ChannelKind,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = sticky_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StickyMessages {
    pub id: i32,
    pub channel_id: i64,
    pub orig_message_id: i64,
    pub bot_message_id: i64,
}

#[derive(Insertable)]
#[diesel(table_name = sticky_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Debug)]
pub struct NewStickyMessage {
    pub channel_id: i64,
    pub orig_message_id: i64,
    pub bot_message_id: i64,
}

#[derive(Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = osu_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OsuUsers {
    pub id: i32,
    pub username: String,
    pub avatar_url: String,
}

#[derive(Hash, PartialEq, Eq, Queryable, Selectable, Associations, Identifiable)]
#[diesel(table_name = osu_user_groups)]
#[diesel(belongs_to(OsuUsers, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OsuUserGroups {
    pub id: i32,
    pub user_id: i32,
    pub member_of: OsuGroup,
}

#[derive(Queryable, Selectable, Associations, Identifiable)]
#[diesel(table_name = osu_user_group_gamemodes)]
#[diesel(belongs_to(OsuUserGroups, foreign_key = user_group_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OsuUserGroupGamemodes {
    pub id: i32,
    pub user_group_id: i32,
    pub gamemode: OsuGamemode,
}
