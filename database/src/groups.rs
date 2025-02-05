use crate::models::{OsuGamemode, OsuUserGroupGamemodes, OsuUsers};
use crate::schema::osu_user_group_gamemodes::dsl::osu_user_group_gamemodes;
use crate::{
    core::{DB, macros::get_conn},
    models::{OsuGroup, OsuUserGroups},
    schema::{self, osu_user_groups::dsl::osu_user_groups, osu_users::dsl::osu_users},
};
use anyhow::Result;
use diesel::{BelongingToDsl, ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use futures::future;
use serde::Deserialize;
use std::collections::HashSet;
use tracing::warn;

// TODO move to backend
#[derive(Debug, Hash, PartialEq, Eq, Clone, Deserialize)]
pub struct OsuUser {
    pub id: i32,
    pub username: String,
    pub avatar_url: String,
    #[serde(rename = "groups", deserialize_with = "deserialize_member_of")]
    pub member_of: Vec<(OsuGroup, Vec<OsuGamemode>)>,
}

// Having to duplicate this struct is icky but no other way around it
#[derive(Debug, PartialEq, Eq)]
pub struct GamemodeUpdate {
    group: OsuGroup,
    added: Vec<OsuGamemode>,
    removed: Vec<OsuGamemode>,
}

fn deserialize_member_of<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<(OsuGroup, Vec<OsuGamemode>)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize, Debug)]
    struct Group {
        // TODO
        // The "alumni" group has `null` as the playmodes hence for the `Option<T>` remove this and deserialize to just a vec
        // The `OsuGroup` struct should use
        identifier: OsuGroup,
        playmodes: Option<Vec<OsuGamemode>>,
    }

    let member_of: Vec<Group> = Vec::deserialize(deserializer)?;
    Ok(member_of
        .iter()
        .map(|group| {
            (
                group.identifier,
                group.playmodes.clone().unwrap_or_default(),
            )
        })
        .collect())
}

pub async fn fetch_all_group_members(group: OsuGroup) -> Result<HashSet<OsuUser>> {
    let mut ret: HashSet<OsuUser> = HashSet::new();
    let conn = get_conn!();

    let group_members = osu_user_groups
        .filter(schema::osu_user_groups::member_of.eq(&group))
        .select(OsuUserGroups::as_select())
        .load(conn)
        .await?;

    for member in group_members {
        let user = osu_users
            .filter(schema::osu_users::id.eq(member.user_id))
            .select(OsuUsers::as_select())
            .first(conn)
            .await?;
        let gamemodes = osu_user_group_gamemodes
            .filter(schema::osu_user_group_gamemodes::user_group_id.eq(member.id))
            .select(OsuUserGroupGamemodes::as_select())
            .load(conn)
            .await?
            .iter()
            .map(|e| e.gamemode)
            .collect::<Vec<_>>();

        ret.insert(OsuUser {
            id: user.id,
            username: user.username,
            avatar_url: user.avatar_url,
            member_of: vec![(group, gamemodes)],
        });
    }

    Ok(ret)
}

pub async fn insert_group_member(
    user: &OsuUsers,
    group: OsuGroup,
    gamemodes: Vec<OsuGamemode>,
) -> Result<()> {
    let conn = get_conn!();
    let user = match osu_users
        .filter(schema::osu_users::dsl::id.eq(user.id))
        .first::<OsuUsers>(conn)
        .await
        .optional()?
    {
        Some(u) => u,
        None => {
            diesel::insert_into(osu_users)
                .values(user)
                .get_result::<OsuUsers>(conn)
                .await?
        }
    };

    if (osu_user_groups
        .filter(schema::osu_user_groups::user_id.eq(user.id))
        .filter(schema::osu_user_groups::member_of.eq(group))
        .first::<OsuUserGroups>(conn)
        .await
        .optional()?)
        .is_some()
    {
        warn!("User is already a member of {group:?} skipping insertion");
        return Ok(());
    }

    let id: i32 = diesel::insert_into(osu_user_groups)
        .values((
            schema::osu_user_groups::dsl::user_id.eq(user.id),
            schema::osu_user_groups::dsl::member_of.eq(group),
        ))
        .get_results::<OsuUserGroups>(conn)
        .await?
        .into_iter()
        .next()
        // TODO remove this
        .unwrap()
        .id;

    for gamemode in gamemodes {
        diesel::insert_into(osu_user_group_gamemodes)
            .values((
                schema::osu_user_group_gamemodes::dsl::user_group_id.eq(&id),
                schema::osu_user_group_gamemodes::dsl::gamemode.eq(gamemode),
            ))
            .execute(conn)
            .await?;
    }

    Ok(())
}

pub async fn update_group_member_gamemodes(user_id: i32, diff: GamemodeUpdate) -> Result<()> {
    let conn = get_conn!();

    let user_group = osu_user_groups
        .filter(schema::osu_user_groups::user_id.eq(user_id))
        .filter(schema::osu_user_groups::member_of.eq(diff.group))
        .select(OsuUserGroups::as_select())
        .first(conn)
        .await?;
    let member_of = OsuUserGroupGamemodes::belonging_to(&user_group)
        .select(OsuUserGroupGamemodes::as_select())
        .load(conn)
        .await?;

    future::join_all(diff.added.iter().map(|g| {
        diesel::insert_into(osu_user_group_gamemodes)
            .values((
                schema::osu_user_group_gamemodes::user_group_id.eq(user_group.id),
                schema::osu_user_group_gamemodes::gamemode.eq(g),
            ))
            .execute(conn)
    }))
        .await;
    future::join_all(
        member_of
            .iter()
            .filter(|i| diff.removed.contains(&i.gamemode))
            .map(|entry| {
                diesel::delete(osu_user_group_gamemodes)
                    .filter(schema::osu_user_group_gamemodes::id.eq(entry.id))
                    .execute(conn)
            }),
    )
        .await;

    Ok(())
}

pub async fn delete_group_member(user_id: i32, group: OsuGroup) -> Result<()> {
    let conn = get_conn!();
    let user = osu_users
        .filter(schema::osu_users::id.eq(user_id))
        .select(OsuUsers::as_select())
        .get_result(conn)
        .await?;

    diesel::delete(
        osu_user_groups
            .filter(schema::osu_user_groups::user_id.eq(user.id))
            .filter(schema::osu_user_groups::member_of.eq(group)),
    )
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn update_osu_user_name(user_id: i32, new_name: String) -> Result<()> {
    diesel::update(osu_users)
        .set(schema::osu_users::username.eq(new_name))
        .execute(get_conn!())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tokio::sync::OnceCell;

    static DB_INIT: OnceCell<()> = OnceCell::const_new();

    async fn init_db() {
        DB_INIT
            .get_or_init(|| async {
                std::env::set_var("DATABASE_URL", "postgres://postgres@127.0.0.1:5432/testing");
                crate::core::initialise()
                    .await
                    .expect("Failed to initialise database");
            })
            .await;
    }

    #[tokio::test]
    async fn fetch_per_group() {
        init_db().await;

        let user1 = OsuUsers {
            id: 727,
            username: "sigidayo".to_string(),
            avatar_url: "https://i.dont.know".to_string(),
        };
        let user2 = OsuUsers {
            id: 728,
            username: "notsigidayo".to_string(),
            avatar_url: "https://i.dont.know2".to_string(),
        };

        insert_group_member(&user1, OsuGroup::BeatmapNominator, vec![
            OsuGamemode::Osu,
            OsuGamemode::Taiko,
        ])
        .await
        .unwrap();
        insert_group_member(&user1, OsuGroup::ProjectLoved, vec![])
            .await
            .unwrap();

        insert_group_member(&user2, OsuGroup::BeatmapNominator, vec![
            OsuGamemode::Mania,
            OsuGamemode::Taiko,
        ])
        .await
        .unwrap();

        let bn_res = fetch_all_group_members(OsuGroup::BeatmapNominator)
            .await
            .unwrap();
        let loved_res = fetch_all_group_members(OsuGroup::ProjectLoved)
            .await
            .unwrap();

        let mut bn_expected = HashSet::with_capacity(2);
        let mut loved_expected = HashSet::with_capacity(1);

        bn_expected.insert(OsuUser {
            id: 727,
            username: "sigidayo".to_string(),
            avatar_url: "https://i.dont.know".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, vec![
                OsuGamemode::Osu,
                OsuGamemode::Taiko,
            ])],
        });
        loved_expected.insert(OsuUser {
            id: 727,
            username: "sigidayo".to_string(),
            avatar_url: "https://i.dont.know".to_string(),
            member_of: vec![(OsuGroup::ProjectLoved, vec![])],
        });

        bn_expected.insert(OsuUser {
            id: 728,
            username: "notsigidayo".to_string(),
            avatar_url: "https://i.dont.know2".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, vec![
                OsuGamemode::Mania,
                OsuGamemode::Taiko,
            ])],
        });

        assert_eq!(bn_expected, bn_res);
        assert_eq!(loved_expected, loved_res);

        delete_group_member(727, OsuGroup::BeatmapNominator)
            .await
            .unwrap();
        bn_expected.remove(&OsuUser {
            id: 727,
            username: "sigidayo".to_string(),
            avatar_url: "https://i.dont.know".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, vec![
                OsuGamemode::Osu,
                OsuGamemode::Taiko,
            ])],
        });

        let bn_res = fetch_all_group_members(OsuGroup::BeatmapNominator)
            .await
            .unwrap();
        let loved_res = fetch_all_group_members(OsuGroup::ProjectLoved)
            .await
            .unwrap();

        assert_eq!(bn_expected, bn_res);
        assert_eq!(loved_expected, loved_res);

        delete_group_member(727, OsuGroup::ProjectLoved)
            .await
            .unwrap();
        delete_group_member(728, OsuGroup::BeatmapNominator)
            .await
            .unwrap();
        bn_expected.drain();
        loved_expected.drain();

        let bn_res = fetch_all_group_members(OsuGroup::BeatmapNominator)
            .await
            .unwrap();
        let loved_res = fetch_all_group_members(OsuGroup::ProjectLoved)
            .await
            .unwrap();

        assert_eq!(bn_expected, bn_res);
        assert_eq!(loved_expected, loved_res);
    }

    #[tokio::test]
    async fn update_gamemodes() {
        init_db().await;

        let user = OsuUsers {
            id: 729,
            username: "sigidayo2".to_string(),
            avatar_url: "https://i.dont.know3".to_string(),
        };

        insert_group_member(&user, OsuGroup::BeatmapNominator, vec![
            OsuGamemode::Osu,
            OsuGamemode::Taiko,
        ])
            .await
            .unwrap();
        update_group_member_gamemodes(729, GamemodeUpdate {
            group: OsuGroup::BeatmapNominator,
            added: vec![OsuGamemode::Fruits],
            removed: vec![OsuGamemode::Taiko],
        })
            .await
            .unwrap();

        let mut expected = HashSet::with_capacity(1);
        expected.insert(OsuUser {
            id: 729,
            username: "sigidayo2".to_string(),
            avatar_url: "https://i.dont.know3".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, vec![
                OsuGamemode::Osu,
                OsuGamemode::Fruits,
            ])],
        });

        let res = fetch_all_group_members(OsuGroup::BeatmapNominator)
            .await
            .unwrap();
        delete_group_member(729, OsuGroup::BeatmapNominator)
            .await
            .unwrap();

        assert_eq!(expected, res);
    }

    #[tokio::test]
    async fn deserialize_group() {
        let json = json!(
            [
                {
                    "avatar_url": "https://a.ppy.sh/8301957?1706078382.jpeg",
                    "id": 8301957,
                    "username": "_gt",
                    "groups": [
                        {
                            "identifier": "bng",
                            "playmodes": [
                                "taiko"
                            ]
                        }
                    ]
                },
                {
                    "avatar_url": "https://a.ppy.sh/6291741?1734789574.jpeg",
                    "id": 6291741,
                    "username": "BlackBN",
                    "groups": [
                        {
                            "identifier": "bng",
                            "playmodes": [
                                "taiko",
                                "fruits"
                            ]
                        }
                    ]
                },
                {
                    "avatar_url": "https://a.ppy.sh/16010604?1731223405.jpeg",
                    "id": 16010604,
                    "username": "Monoseul",
                    "groups": [
                        {
                            "identifier": "bng",
                            "playmodes": [
                                "mania"
                            ]
                        },
                        {
                            "identifier": "loved",
                            "playmodes": []
                        }
                    ]
                },
                {
                    "avatar_url": "https://a.ppy.sh/1653229?1723014420.jpeg",
                    "id": 1653229,
                    "username": "_Stan",
                    "groups": [
                        {
                            "identifier": "bng",
                            "playmodes": [
                                "mania"
                            ]
                        },
                        {
                            "identifier": "alumni",
                            "playmodes": []
                        },
                    ]
                }
            ]
        );

        let users: Vec<OsuUser> = match serde_json::from_value(json) {
            Ok(u) => u,
            Err(e) => panic!("{}", e),
        };

        let expected = vec![
            OsuUser {
                id: 8301957,
                username: "_gt".to_string(),
                avatar_url: "https://a.ppy.sh/8301957?1706078382.jpeg".to_string(),
                member_of: vec![(OsuGroup::BeatmapNominator, vec![OsuGamemode::Taiko])],
            },
            OsuUser {
                id: 6291741,
                username: "BlackBN".to_string(),
                avatar_url: "https://a.ppy.sh/6291741?1734789574.jpeg".to_string(),
                member_of: vec![(OsuGroup::BeatmapNominator, vec![
                    OsuGamemode::Taiko,
                    OsuGamemode::Fruits,
                ])],
            },
            OsuUser {
                id: 16010604,
                username: "Monoseul".to_string(),
                avatar_url: "https://a.ppy.sh/16010604?1731223405.jpeg".to_string(),
                member_of: vec![
                    (OsuGroup::BeatmapNominator, vec![OsuGamemode::Mania]),
                    (OsuGroup::ProjectLoved, vec![]),
                ],
            },
            OsuUser {
                id: 1653229,
                username: "_Stan".to_string(),
                avatar_url: "https://a.ppy.sh/1653229?1723014420.jpeg".to_string(),
                member_of: vec![
                    (OsuGroup::BeatmapNominator, vec![OsuGamemode::Mania]),
                    (OsuGroup::Alumni, vec![]),
                ],
            },
        ];

        assert_eq!(users, expected);
    }
}
