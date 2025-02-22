use crate::REQWEST_CLIENT;
use anyhow::{Context, Result, anyhow};
use common::context::get_context_wrapper;
use database::subscriptions::{ChannelType, SubscriptionMode, fetch_all_subscribed_channels};
use database::{
    groups::{
        GamemodeUpdate, OsuUser, delete_group_member, fetch_all_group_members, insert_group_member,
        update_group_member_gamemodes, update_osu_user_profile,
    },
    models::OsuGroup,
};
use futures::future;
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage},
    model::{Colour, Timestamp, id::ChannelId},
};
use smallvec::SmallVec;
use std::collections::HashSet;
use tokio::{
    task,
    time::{self, Duration},
};
use tracing::{debug, error, info, instrument};

const SLEEP_DURATION: Duration = Duration::from_hours(4);
const ERROR_COOLDOWN: Duration = Duration::from_mins(1);

const TRACKED_GROUPS: [OsuGroup; 8] = [
    OsuGroup::BeatmapNominator,
    OsuGroup::ProbationaryBeatmapNominator,
    OsuGroup::NominationAssessmentTeam,
    OsuGroup::GlobalModerationTeam,
    OsuGroup::Developer,
    OsuGroup::FeatureArtist,
    OsuGroup::BeatmapSpotlightCurator,
    OsuGroup::ProjectLoved,
];

const START_HTML_TAG: &str = r#"<script id="json-users" type="application/json">"#;
const END_HTML_TAG: &str = r#"</script>"#;

pub struct GroupManager;
impl GroupManager {
    pub fn new() -> Self {
        info!("Spawning new group manager");
        task::spawn(async move {
            for group in TRACKED_GROUPS {
                task::spawn(async move {
                    if let Err(why) = update_group(group).await {
                        error!("Failed to update {:?}: {:?}", group, why);
                        time::sleep(ERROR_COOLDOWN).await;
                    }
                });
            }
            time::sleep(SLEEP_DURATION).await;
        });

        Self
    }
}

impl Default for GroupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Diff {
    added: Vec<OsuUser>,
    removed: Vec<OsuUser>,
    updated: Vec<(OsuUser, GamemodeUpdate)>,
}

#[instrument]
async fn update_group(group: OsuGroup) -> Result<()> {
    info!("Updating group");
    let html = REQWEST_CLIENT
        .get(format!("https://osu.ppy.sh/groups/{}", group.id()))
        .send()
        .await
        .context("Failed to send request to osu! api")?
        .text()
        .await
        .context("Failed to get text from HTTP response")?;
    let json = extract_json(&html)?;

    let external_data: Vec<OsuUser> =
        serde_json::from_str(json).map_err(|err| anyhow!("Failed to parse JSON {}", err))?;
    let external_data: HashSet<OsuUser> = HashSet::from_iter(external_data);
    let local_data = fetch_all_group_members(group).await?;
    let populate_flag = local_data.is_empty();

    for (user_id, new_name, new_pfp) in update_profiles(&local_data, &external_data) {
        update_osu_user_profile(user_id, new_name, new_pfp).await?;
    }

    let diff = get_diff(local_data, HashSet::from_iter(external_data), group);
    let embeds = process(&diff, group).await?;

    let channels =
        fetch_all_subscribed_channels(ChannelType::Groups(SubscriptionMode::Subscribe)).await?;
    //let channels = ChannelId::new(TEST_CHANNEL);
    let ctx = get_context_wrapper();

    debug!("New users {:#?}", diff.added);
    debug!("Removed users {:#?}", diff.removed);
    debug!("Updated users {:#?}", diff.updated);

    if !populate_flag {
        let embed_count = embeds.len();

        for channel in channels {
            for embed in &embeds {
                // Why don't you accept a pointer grr
                ChannelId::new(channel as u64)
                    .send_message(&ctx, CreateMessage::new().embed(embed.clone()))
                    .await?;
            }
        }
        if embed_count > 0 {
            info!("Finished sending {} messages", embed_count);
        }
    } else {
        info!("Populating database, skipping messages");
    }

    // Cleanup
    future::join_all(diff.added.iter().map(|i| insert_group_member(i, group))).await;
    future::join_all(
        diff.removed
            .iter()
            .map(|i| delete_group_member(i.id, group)),
    )
    .await;
    future::join_all(
        diff.updated
            .iter()
            .map(|i| update_group_member_gamemodes(i.0.id, &i.1)),
    )
    .await;
    debug!("Cleanup completed");

    Ok(())
}

async fn process(diff: &Diff, group: OsuGroup) -> Result<Vec<CreateEmbed>> {
    let mut embeds: Vec<CreateEmbed> = Vec::new();
    for user in &diff.added {
        let mut description = String::new();
        if let Some(gamemodes_iter) = user
            .member_of
            .iter()
            .find(|(in_group, gamemodes)| *in_group == group && !gamemodes.is_empty())
            .map(|(_, gamemodes)| gamemodes.iter())
        {
            description.push_str("```diff\n");
            for gamemode in gamemodes_iter {
                description.push_str(&format!("+ {gamemode}\n"))
            }
            description.push_str("```");
        }

        embeds.push(
            CreateEmbed::default()
                .author(
                    CreateEmbedAuthor::new(&user.username)
                        .icon_url(&user.avatar_url)
                        .url(format!("https://osu.ppy.sh/users/{}", &user.id)),
                )
                .title(format!("Added to `{group}`"))
                .description(description)
                .colour(Colour::new(0x80ff80))
                .footer(CreateEmbedFooter::new("Made with ♥"))
                .timestamp(Timestamp::now()),
        )
    }

    for user in &diff.removed {
        embeds.push(
            CreateEmbed::default()
                .author(
                    CreateEmbedAuthor::new(&user.username)
                        .icon_url(&user.avatar_url)
                        .url(format!("https://osu.ppy.sh/users/{}", &user.id)),
                )
                .title(format!("Removed from `{group}`"))
                .colour(Colour::new(0xff3737))
                .footer(CreateEmbedFooter::new("Made with ♥"))
                .timestamp(Timestamp::now()),
        )
    }

    for (user, update) in &diff.updated {
        if update.group != group {
            continue;
        }
        let mut description = String::new();
        description.push_str("```diff\n");
        update
            .added
            .iter()
            .for_each(|i| description.push_str(&format!("+ {i}\n")));
        update
            .removed
            .iter()
            .for_each(|i| description.push_str(&format!("- {i}\n")));
        description.push_str("```");

        embeds.push(
            CreateEmbed::default()
                .author(
                    CreateEmbedAuthor::new(&user.username)
                        .icon_url(&user.avatar_url)
                        .url(format!("https://osu.ppy.sh/users/{}", &user.id)),
                )
                .title(format!("Updated gamemodes in `{group}`"))
                .description(description)
                .colour(Colour::new(0xffff80))
                .footer(CreateEmbedFooter::new("Made with ♥"))
                .timestamp(Timestamp::now()),
        )
    }

    Ok(embeds)
}

// This would be so much less verbose without trying to avoid hashing types
fn get_diff(
    stored_data: HashSet<OsuUser>,
    external_data: HashSet<OsuUser>,
    for_group: OsuGroup,
) -> Diff {
    let diff_users = |left: &HashSet<OsuUser>, right: &HashSet<OsuUser>| {
        left.iter()
            .filter(|user| !right.iter().any(|r| r.id == user.id))
            .cloned()
            .collect::<Vec<OsuUser>>()
    };

    let new_users = diff_users(&external_data, &stored_data);
    let removed_users = diff_users(&stored_data, &external_data);

    let updated_users: Vec<(OsuUser, GamemodeUpdate)> = external_data
        .into_iter()
        .filter_map(|api_user| {
            let stored_user = stored_data.iter().find(|u| u.id == api_user.id)?;
            for (api_group, api_modes) in &api_user.member_of {
                if let Some((_, stored_modes)) = stored_user
                    .member_of
                    .iter()
                    .find(|(group, _)| group == api_group)
                {
                    let added = SmallVec::from_iter(
                        api_modes
                            .iter()
                            .filter(|item| !stored_modes.contains(item))
                            .copied(),
                    );
                    let removed = SmallVec::from_iter(
                        stored_modes
                            .iter()
                            .filter(|item| !api_modes.contains(item))
                            .copied(),
                    );
                    if !added.is_empty() || !removed.is_empty() {
                        return Some((
                            api_user,
                            GamemodeUpdate {
                                group: for_group,
                                added,
                                removed,
                            },
                        ));
                    };
                }
            }

            None
        })
        .collect();

    Diff {
        added: new_users,
        removed: removed_users,
        updated: updated_users,
    }
}

fn update_profiles<'a>(
    local_users: &'a HashSet<OsuUser>,
    remote_users: &'a HashSet<OsuUser>,
) -> impl Iterator<Item = (i32, Option<String>, Option<String>)> + 'a {
    remote_users.iter().filter_map(|user| {
        let shared = local_users.iter().find(|u| u.id == user.id)?;
        let name_update = (shared.username != user.username).then_some(user.username.clone());
        let avatar_update =
            (shared.avatar_url != user.avatar_url).then_some(user.avatar_url.clone());

        (name_update.is_some() || avatar_update.is_some()).then_some((
            user.id,
            name_update,
            avatar_update,
        ))
    })
}

fn extract_json(html: &str) -> Result<&str> {
    let start_idx = html
        .find(START_HTML_TAG)
        .ok_or_else(|| anyhow!("Cannot find HTML starting tag for group"))?;
    let content = start_idx + START_HTML_TAG.len();

    let end_idx = html[content..]
        .find(END_HTML_TAG)
        .ok_or_else(|| anyhow!("Cannot find HTML ending tag for group"))?;

    Ok(html[content..content + end_idx].trim())
}

#[cfg(test)]
mod test {
    use super::*;
    use database::models::OsuGamemode;
    use pretty_assertions::assert_eq;
    use smallvec::smallvec;

    #[test]
    fn diff_calc_username_update() {
        let mut stored = HashSet::new();
        let mut api = HashSet::new();

        stored.insert(OsuUser {
            id: 100,
            username: "original_name".to_string(),
            avatar_url: "placeholder".to_string(),
            member_of: vec![(
                OsuGroup::BeatmapNominator,
                smallvec![OsuGamemode::Osu, OsuGamemode::Taiko,],
            )],
        });
        api.insert(OsuUser {
            id: 100,
            username: "changed_name".to_string(),
            avatar_url: "changed_avatar".to_string(),
            member_of: vec![(
                OsuGroup::BeatmapNominator,
                smallvec![OsuGamemode::Osu, OsuGamemode::Taiko,],
            )],
        });

        let diff = get_diff(stored, api, OsuGroup::BeatmapNominator);

        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.updated.is_empty());
    }

    #[test]
    fn diff_calc_role_update() {
        let mut stored = HashSet::new();
        let mut api = HashSet::new();

        stored.insert(OsuUser {
            id: 1,
            username: "a".to_string(),
            avatar_url: "b".to_string(),
            member_of: vec![
                (
                    OsuGroup::BeatmapNominator,
                    smallvec![OsuGamemode::Osu, OsuGamemode::Taiko,],
                ),
                (
                    OsuGroup::BeatmapSpotlightCurator,
                    smallvec![OsuGamemode::Osu, OsuGamemode::Mania,],
                ),
            ],
        });
        api.insert(OsuUser {
            id: 1,
            username: "a".to_string(),
            avatar_url: "b".to_string(),
            member_of: vec![(
                OsuGroup::BeatmapSpotlightCurator,
                smallvec![OsuGamemode::Taiko],
            )],
        });

        let diff = get_diff(stored, api, OsuGroup::BeatmapNominator);

        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(
            vec![(
                OsuUser {
                    id: 1,
                    username: "a".to_string(),
                    avatar_url: "b".to_string(),
                    member_of: vec![(
                        OsuGroup::BeatmapSpotlightCurator,
                        smallvec![OsuGamemode::Taiko]
                    )],
                },
                GamemodeUpdate {
                    group: OsuGroup::BeatmapNominator,
                    added: smallvec![OsuGamemode::Taiko],
                    removed: smallvec![OsuGamemode::Osu, OsuGamemode::Mania]
                }
            )],
            diff.updated
        );
    }

    #[test]
    fn diff_calc_gamemode_update() {
        let mut stored = HashSet::new();
        let mut api = HashSet::new();

        stored.insert(OsuUser {
            id: 1,
            username: "a".to_string(),
            avatar_url: "b".to_string(),
            member_of: vec![(
                OsuGroup::BeatmapNominator,
                smallvec![OsuGamemode::Osu, OsuGamemode::Taiko,],
            )],
        });
        api.insert(OsuUser {
            id: 1,
            username: "a".to_string(),
            avatar_url: "b".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
        });

        let diff = get_diff(stored, api, OsuGroup::BeatmapNominator);

        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(
            vec![(
                OsuUser {
                    id: 1,
                    username: "a".to_string(),
                    avatar_url: "b".to_string(),
                    member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
                },
                GamemodeUpdate {
                    group: OsuGroup::BeatmapNominator,
                    added: smallvec![],
                    removed: smallvec![OsuGamemode::Taiko]
                }
            )],
            diff.updated
        );
    }

    #[test]
    fn update_user_profile() {
        let mut stored = HashSet::new();
        let mut api = HashSet::new();

        stored.insert(OsuUser {
            id: 500,
            username: "old_name".to_string(),
            avatar_url: "placeholder".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
        });
        stored.insert(OsuUser {
            id: 501,
            username: "dummy_user".to_string(),
            avatar_url: "placeholder".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
        });
        stored.insert(OsuUser {
            id: 503,
            username: "old_name".to_string(),
            avatar_url: "placeholder".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
        });
        api.insert(OsuUser {
            id: 500,
            username: "new_name".to_string(),
            avatar_url: "placeholder".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
        });
        api.insert(OsuUser {
            id: 502,
            username: "dummy_user".to_string(),
            avatar_url: "placeholder".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
        });
        api.insert(OsuUser {
            id: 503,
            username: "new_name".to_string(),
            avatar_url: "new_pfp".to_string(),
            member_of: vec![(OsuGroup::BeatmapNominator, smallvec![OsuGamemode::Osu])],
        });

        let updated = update_profiles(&stored, &api).collect::<Vec<_>>();
        assert_eq!(
            &(500, Some("new_name".to_string()), None),
            updated.iter().find(|i| i.0 == 500).unwrap()
        );
        assert_eq!(
            &(
                503,
                Some("new_name".to_string()),
                Some("new_pfp".to_string())
            ),
            updated.iter().find(|i| i.0 == 503).unwrap()
        );
    }
}
