#![allow(dead_code)]

use crate::schema::{beatmapset_subscriptions, beatmapsets, subscriptions};
use diesel::{
    AsExpression, Associations, FromSqlRow, Identifiable, Queryable, Selectable,
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    serialize::{IsNull, Output, ToSql},
};
use std::io::Write;

#[derive(Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::ChannelKind)]
pub enum ChannelKind {
    Mapfeed,
    Music,
}

impl ToSql<crate::schema::sql_types::ChannelKind, Pg> for ChannelKind {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match *self {
            ChannelKind::Mapfeed => out.write_all(b"mapfeed")?,
            ChannelKind::Music => out.write_all(b"music")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::ChannelKind, Pg> for ChannelKind {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"mapfeed" => Ok(ChannelKind::Mapfeed),
            b"music" => Ok(ChannelKind::Music),
            _ => Err("Unrecognized enum variant".into()),
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
