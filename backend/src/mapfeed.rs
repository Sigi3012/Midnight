use crate::{
    api::{
        osu::{fetch_all_qualified_maps, fetch_beatmaps, BeatmapsetVec},
        types::{BeatmapStatus, Beatmapset, Modes},
    },
    types::SubscriptionError,
};
use anyhow::{anyhow, Error};
use common::context::ContextWrapper;
use common::{context::get_context_wrapper, math::mode};
use database::{
    mapfeed::{
        delete_beatmap, fetch_all_ids, fetch_all_subscribers, insert_beatmap, insert_beatmaps,
        subscribe_to_beatmap, unsubscribe_from_beatmap,
    },
    subscriptions::{
        fetch_all_subscribed_channels, ChannelType, SubscriptionMode, UserAdditionStatus,
        UserDeletionStatus,
    },
};
use fancy_regex::Regex;
use futures::future::join_all;
use itertools::{EitherOrBoth, Itertools};
use log::{debug, error, info, warn};
use poise::{serenity_prelude as serenity, CreateReply};
use serenity::all::ComponentInteraction;
use serenity::{
    all::{CreateInteractionResponse, CreateInteractionResponseMessage},
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
    futures::StreamExt,
    model::{colour::Colour, id::ChannelId},
};
use smallvec::SmallVec;
use std::{
    collections::HashSet,
    fmt::Display,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    task,
    time::{sleep, Duration, Instant},
};

const FIFTEEN_MINUTES: Duration = Duration::from_secs(60 * 15);
const ERROR_COOLDOWN: Duration = Duration::from_secs(60 * 3);
const BUTTON_TIMEOUT: Duration = Duration::from_secs(60 * 120);

pub struct MapfeedManager {
    stop_flag: Arc<AtomicBool>,
}

struct MessageData<'a> {
    embed: CreateEmbed,
    subscribed_user_ids: Option<Vec<i64>>,
    beatmapset_data: &'a Beatmapset,
}

struct ButtonInteraction {
    id: i32,
    state: ButtonState,
}

enum ButtonState {
    Subscribe,
    Unsubscribe,
}

lazy_static! {
    #[derive(Debug)]
    static ref OSU_LINK_REGEX: Regex = Regex::new(r#"(?:https:\/\/osu\.ppy\.sh/beatmapsets/)(\d+)"#).expect("Regex should compile");
}

impl MapfeedManager {
    pub async fn start() -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let clone = stop_flag.clone();

        info!("Spawning new mapfeed manager");
        task::spawn(async move {
            while !clone.load(Ordering::Relaxed) {
                if let Err(why) = update_mapfeed().await {
                    error!("Failed to refresh mapfeed, error: {}", why);
                    sleep(ERROR_COOLDOWN).await;
                } else {
                    sleep(FIFTEEN_MINUTES).await;
                }
            }

            warn!("Mapfeed stopped")
        });

        MapfeedManager { stop_flag }
    }

    pub async fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

impl Display for ButtonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subscribe => write!(f, "subscribed"),
            Self::Unsubscribe => write!(f, "unsubscribed"),
        }
    }
}

// FIXME this really needs error handling
async fn update_mapfeed() -> Result<(), Error> {
    let start_time = Instant::now();

    info!("Fetching remote ids");
    let remote_ids = fetch_all_qualified_maps().await?;

    info!("Fetching local ids");
    let local_ids: Vec<i32> = fetch_all_ids().await?.unwrap_or_else(Vec::new);

    let remote_ids_hashset: HashSet<i32> = remote_ids.into_iter().collect();
    let local_ids_hashset: HashSet<i32> = local_ids.into_iter().collect();

    let new_maps: BeatmapsetVec = {
        let ids = remote_ids_hashset
            .difference(&local_ids_hashset)
            .cloned()
            .collect();

        fetch_beatmaps(ids).await?
    };
    let changed_maps: BeatmapsetVec = {
        let ids = local_ids_hashset
            .difference(&remote_ids_hashset)
            .cloned()
            .collect();

        match fetch_beatmaps(ids).await {
            Ok(beatmapsets) => beatmapsets,
            Err(e) => {
                error!(
                    "Something went wrong while fetching beatmapset struct: {}",
                    e
                );
                return Err(anyhow!("Failed to fetch beatmapset struct"));
            }
        }
    };
    let common_ids: Vec<i32> = remote_ids_hashset
        .intersection(&local_ids_hashset)
        .cloned()
        .collect();

    info!(
        "New maps: {:?}",
        new_maps.iter().map(|map| map.id).collect::<Vec<i32>>()
    );
    info!(
        "Changed maps: {:?}",
        changed_maps.iter().map(|map| map.id).collect::<Vec<i32>>()
    );
    info!("Common ids: {:?}", common_ids);

    let channels = fetch_all_subscribed_channels(ChannelType::Mapfeed(SubscriptionMode::Subscribe))
        .await?
        .unwrap_or_else(Vec::new);
    debug!("Channel ids {:?}", channels);

    // I wrote this functionally mostly just for fun, I definitely think a for loop based approach is better.
    let message_data = join_all(
        new_maps
            .iter()
            .zip_longest(changed_maps.iter())
            .map(|pair| match pair {
                EitherOrBoth::Both(x, y) => (Some(x), Some(y)),
                EitherOrBoth::Left(x) => (Some(x), None),
                EitherOrBoth::Right(y) => (None, Some(y)),
            })
            .collect::<Vec<(Option<&Beatmapset>, Option<&Beatmapset>)>>()
            .iter()
            .map(|(new_map, changed_map)| {
                let new_map = *new_map;
                let changed_map = *changed_map;
                async move {
                    let mut ret = SmallVec::<[MessageData; 2]>::new();

                    if let Some(m) = new_map {
                        let embed = build_embed(m);

                        if let Err(why) = insert_beatmap(m.id).await {
                            error!(
                            "ID: {} failed to insert into database, skipping insertion. Error: {}",
                            m.id, why
                        )
                        } else {
                            debug!("Inserted ID: {}", m.id)
                        };
                        ret.push(MessageData {
                            embed,
                            subscribed_user_ids: None,
                            beatmapset_data: m,
                        })
                    };
                    if let Some(m) = changed_map {
                        let embed = build_embed(m);
                        match fetch_all_subscribers(m.id).await {
                            Ok(option) => {
                                if let Some(ids) = option {
                                    ret.push(MessageData {
                                        embed,
                                        subscribed_user_ids: Some(ids),
                                        beatmapset_data: m,
                                    });
                                } else {
                                    ret.push(MessageData {
                                        embed,
                                        subscribed_user_ids: None,
                                        beatmapset_data: m,
                                    });
                                }
                                clean_up_beatmap(m.id).await;
                            }
                            Err(e) => {
                                error!("Failed to fetch subscribers for pk: {}, error: {}", m.id, e)
                            }
                        }
                    }

                    ret
                }
            })
            .collect::<Vec<_>>(),
    )
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<MessageData>>();

    info!("Sending {} unique messages", message_data.len());

    // Sending messages in every subscribed channel
    for channel_id in channels {
        let channel = ChannelId::new(channel_id as u64);

        for message in message_data.iter() {
            if let Err(why) = message_handler(channel, message).await {
                error!(
                    "Something went wrong while building and sending message, {}",
                    why
                )
            };
        }
    }

    let duration = start_time.elapsed();
    info!("Mapfeed cycle took {:?} seconds", duration);
    Ok(())
}

pub fn build_embed(beatmapset: &Beatmapset) -> CreateEmbed {
    let mapper_url = beatmapset.mapper.replace(' ', "%20");
    let most_common_mode = {
        let modes: Vec<&Modes> = beatmapset
            .beatmaps
            .iter()
            .map(|beatmap| &beatmap.mode)
            .collect();

        mode(&modes).unwrap_or(&Modes::Standard)
    };

    let star_rating_display_string = {
        if beatmapset.beatmaps.len() == 1 {
            format!(
                "{} \u{2605} \u{2022} 1 Difficulty",
                beatmapset.beatmaps[0].star_rating
            )
        } else {
            let ratings_vec: Vec<&f32> = beatmapset
                .beatmaps
                .iter()
                .map(|beatmap| &beatmap.star_rating)
                .collect();

            let min_rating = ratings_vec
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let max_rating = ratings_vec
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            format!(
                "{} - {} \u{2605} \u{2022} {} Difficulties",
                min_rating.unwrap_or(&&0.0),
                max_rating.unwrap_or(&&0.0),
                beatmapset.beatmaps.len()
            )
        }
    };

    let mut ranked_date_string: String = "**".to_string();
    if let Some(unix) = beatmapset.ranked_date_unix {
        ranked_date_string = format!(" <t:{}:R>**", unix)
    };

    #[allow(clippy::unwrap_used)] // The `submitted_date_unix` field will never be `None`
    let description = format!(
        "**[{}](https://osu.ppy.sh/beatmapsets/{})** | **{}{}\nMapped by [{}](https://osu.ppy.sh/users/{}) | [{}]\nArtist: {}\nSubmitted: <t:{}:R>\n\n{}",
        beatmapset.title,
        beatmapset.id,
        beatmapset.ranked_status,
        ranked_date_string,
        beatmapset.mapper,
        mapper_url,
        most_common_mode,
        beatmapset.artist,
        beatmapset.submitted_date_unix.unwrap(),
        star_rating_display_string
    );
    let colour = match &beatmapset.ranked_status {
        // This is marked explicitly on purpose, do not wildcard match the colours
        BeatmapStatus::Ranked => Colour::from_rgb(64, 90, 201), // 🟦
        BeatmapStatus::Qualified => Colour::from_rgb(209, 160, 61), // 🟧
        BeatmapStatus::Loved => Colour::from_rgb(255, 105, 180), // Pink (there was no square)
        BeatmapStatus::Pending | BeatmapStatus::Wip | BeatmapStatus::Graveyard => {
            Colour::from_rgb(210, 43, 43) // 🟥,
        }
    };
    let image = format!(
        "https://assets.ppy.sh/beatmaps/{}/covers/card.jpg",
        beatmapset.id
    );
    CreateEmbed::new()
        .description(description)
        .colour(colour)
        .image(image)
}

async fn clean_up_beatmap(id: i32) {
    if let Err(why) = delete_beatmap(id).await {
        error!(
            "ID: {} failed to be deleted from the database, skipping deletion. Error: {}",
            id, why
        )
    } else {
        debug!("Deleted ID: {}", id)
    };
}

#[allow(clippy::unwrap_used)]
fn parse_custom_button_id(s: &str) -> ButtonInteraction {
    let mut i = s.split('.').collect::<SmallVec<[&str; 2]>>().into_iter();
    let id: i32 = i.next().unwrap().parse().unwrap();
    let state = match i.next().unwrap() {
        "subscribe" => ButtonState::Subscribe,
        "unsubscribe" => ButtonState::Unsubscribe,
        _ => unreachable!(),
    };
    ButtonInteraction { id, state }
}

async fn message_handler(
    target: ChannelId,
    message_data: &MessageData<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = get_context_wrapper();

    if message_data.beatmapset_data.ranked_status == BeatmapStatus::Qualified {
        let components = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(format!("{}.subscribe", message_data.beatmapset_data.id))
                .label("Subscribe")
                .style(serenity::ButtonStyle::Primary),
            serenity::CreateButton::new(format!("{}.unsubscribe", message_data.beatmapset_data.id))
                .label("Unsubscribe")
                .style(serenity::ButtonStyle::Danger),
        ]);
        let builder = match &message_data.subscribed_user_ids {
            Some(ids) => {
                let pings: Vec<String> = ids.iter().map(|id| format!("<@{}>", id)).collect();
                CreateMessage::new()
                    .content(pings.join(", "))
                    .embed(message_data.embed.clone())
                    .components(vec![components])
            }
            None => CreateMessage::new()
                .embed(message_data.embed.clone())
                .components(vec![components]),
        };
        let message = target.send_message(ctx, builder).await?;

        tokio::spawn(async move {
            let mut interaction_stream = message
                .await_component_interaction(&ctx.shard)
                .timeout(BUTTON_TIMEOUT)
                .stream();

            while let Some(interaction) = interaction_stream.next().await {
                handle_interaction(interaction, ctx).await;
            }
        });
    } else {
        let builder = match &message_data.subscribed_user_ids {
            Some(ids) => {
                let pings: Vec<String> = ids.iter().map(|id| format!("<@{}>", id)).collect();
                CreateMessage::new()
                    .content(pings.join(", "))
                    .embed(message_data.embed.clone())
            }
            None => CreateMessage::new().embed(message_data.embed.clone()),
        };
        target.send_message(ctx, builder).await?;
    }

    Ok(())
}

async fn handle_interaction(interaction: ComponentInteraction, ctx: &ContextWrapper) {
    let content: &str;

    let parsed = parse_custom_button_id(&interaction.data.custom_id);
    match parsed.state {
        ButtonState::Subscribe => {
            match subscribe_to_beatmap(interaction.user.id.get() as i64, parsed.id).await {
                Ok(status) => match status {
                    UserAdditionStatus::UserAdded => content = "Subscribed successfully",
                    UserAdditionStatus::UserAlreadyExists => {
                        content = "You are already subscribed to this beatmap!"
                    }
                },
                Err(e) => {
                    error!(
                        "Something went wrong while subscribing to ID: {}, error: {}",
                        parsed.id, e
                    );
                    return;
                }
            }
        }
        ButtonState::Unsubscribe => {
            match unsubscribe_from_beatmap(interaction.user.id.get() as i64, parsed.id).await {
                Ok(status) => match status {
                    UserDeletionStatus::UserRemoved => content = "Unsubscribed successfully",
                    UserDeletionStatus::UserDoesNotExist => {
                        content = "You are not subscribed to this beatmap!"
                    }
                },
                Err(e) => {
                    error!(
                        "Something went wrong while unsubscribing from ID: {}, error: {}",
                        parsed.id, e
                    );
                    return;
                }
            }
        }
    }

    match interaction
        .create_response(
            &ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .ephemeral(true)
                    .content(content),
            ),
        )
        .await
    {
        Ok(_) => (),
        Err(e) => error!("Interaction failure, {}", e),
    }
}

pub async fn populate() -> anyhow::Result<()> {
    info!("Populating database");
    if fetch_all_ids().await?.is_none() {
        let ids = fetch_all_qualified_maps().await?;
        insert_beatmaps(ids).await?;
    };
    Ok(())
}

pub async fn subscription_handler(
    subscriber: i64,
    link: &str,
    mode: SubscriptionMode,
) -> Result<(), SubscriptionError> {
    if !OSU_LINK_REGEX.is_match(link)? {
        return Err(SubscriptionError::InvalidLink);
    }
    debug!("{:?}", OSU_LINK_REGEX.captures(link)?);
    let id = match OSU_LINK_REGEX.captures(link)? {
        Some(capture) => match capture.get(1) {
            Some(id) => id.as_str().parse::<i32>()?,
            None => return Err(SubscriptionError::NonCapture),
        },
        None => return Err(SubscriptionError::InvalidLink),
    };
    match mode {
        SubscriptionMode::Subscribe => {
            subscribe_to_beatmap(subscriber, id).await?;
        }
        SubscriptionMode::Unsubscribe => {
            unsubscribe_from_beatmap(subscriber, id).await?;
        }
    };

    Ok(())
}

pub fn create_reply_with_sorted_beatmaps(mut beatmaps: BeatmapsetVec) -> CreateReply {
    beatmaps.sort_by(|a, b| a.ranked_date_unix.cmp(&b.ranked_date_unix));

    CreateReply::default().ephemeral(true).embed(
        CreateEmbed::default()
            .title("Beatmaps you are subscribed to")
            .color(Colour::new(0x6758b8))
            .description(format!(
                "- {}",
                beatmaps
                    .iter()
                    .map(|b| format!("[{}](https://osu.ppy.sh/beatmapsets/{})", b.title, b.id))
                    .collect::<Vec<String>>()
                    .join("\n- ")
            ))
            .footer(CreateEmbedFooter::new(
                "Sorted closest to being ranked to furthest",
            )),
    )
}
