use crate::{
    api::{
        osu::{fetch_all_qualified_maps, fetch_beatmaps},
        types::{BeatmapStatus, Beatmapset, Modes},
    },
    types::SubscriptionError,
};
use common::{context::get_context_wrapper, math::mode};
use database::mapfeed::{
    delete_beatmap, fetch_all_ids, fetch_all_subscribed_channels, fetch_all_subscribers,
    insert_beatmap, insert_beatmaps, subscribe_to_beatmap, unsubscribe_from_beatmap,
};
use fancy_regex::Regex;
use log::{debug, error, info, warn};
use poise::{serenity_prelude as serenity, CreateReply};
use serenity::{
    all::{CreateInteractionResponse, CreateInteractionResponseMessage},
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
    futures::StreamExt,
    model::{colour::Colour, id::ChannelId},
};
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
    static ref OSU_LINK_REGEX: Regex = Regex::new(r#"(?:https:\/\/osu\.ppy\.sh/beatmapsets/)(\d+)"#).unwrap();
}

impl MapfeedManager {
    pub async fn start() -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let clone = stop_flag.clone();

        info!("Spawning new mapfeed manager");
        task::spawn(async move {
            while !clone.load(Ordering::Relaxed) {
                update_mapfeed().await;
                sleep(FIFTEEN_MINUTES).await;
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
async fn update_mapfeed() {
    let start_time = Instant::now();

    info!("Fetching remote ids");
    let remote_ids = fetch_all_qualified_maps().await.unwrap();

    info!("Fetching local ids");
    let local_ids = {
        match fetch_all_ids().await.unwrap() {
            Some(ids) => ids,
            None => Vec::new(),
        }
    };

    let remote_ids_hashset: HashSet<i32> = remote_ids.into_iter().collect();
    let local_ids_hashset: HashSet<i32> = local_ids.into_iter().collect();

    let new_maps: Vec<Beatmapset> = {
        let ids = remote_ids_hashset
            .difference(&local_ids_hashset)
            .cloned()
            .collect();

        match fetch_beatmaps(ids).await {
            Ok(beatmapsets) => beatmapsets,
            Err(e) => {
                error!(
                    "Something went wrong while fetching beatmapset struct: {}",
                    e
                );
                return;
            }
        }
    };
    let changed_maps: Vec<Beatmapset> = {
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
                return;
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

    match fetch_all_subscribed_channels().await {
        Ok(channels) => {
            if let None = channels {
                info!("No channels are subscribed to the mapfeed, exiting.");
                return;
            }
            // NOTE unwrapping here is fine because we already check if its None
            let channels = channels.unwrap();
            debug!("Channel ids {:?}", channels);

            // Building message embeds
            let mut message_data: Vec<MessageData> = Vec::new();
            for beatmapset in new_maps.iter() {
                let embed = build_embed(beatmapset).await;
                message_data.push(MessageData {
                    embed,
                    subscribed_user_ids: None,
                    beatmapset_data: &beatmapset,
                });

                if let Err(why) = insert_beatmap(beatmapset.id).await {
                    error!(
                        "ID: {} failed to insert into database, skipping insertion. Error: {}",
                        beatmapset.id, why
                    )
                } else {
                    debug!("Inserted ID: {}", beatmapset.id)
                };
            }
            for beatmapset in changed_maps.iter() {
                let embed = build_embed(beatmapset).await;
                match fetch_all_subscribers(beatmapset.id).await {
                    Ok(option) => {
                        if let Some(ids) = option {
                            message_data.push(MessageData {
                                embed,
                                subscribed_user_ids: Some(ids),
                                beatmapset_data: &beatmapset,
                            });
                            clean_up_beatmap(beatmapset.id).await;
                        } else {
                            message_data.push(MessageData {
                                embed,
                                subscribed_user_ids: None,
                                beatmapset_data: &beatmapset,
                            });
                            clean_up_beatmap(beatmapset.id).await;
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to fetch subscribers for pk: {}, error: {}",
                            beatmapset.id, e
                        )
                    }
                }
            }
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
            info!("Mapfeed cycle took {:?} seconds", duration)
        }
        Err(why) => {
            error!(
                "Encounted an error while fetching subscribed channels: {}",
                why
            );
            return;
        }
    }
}

pub async fn build_embed(beatmapset: &Beatmapset) -> CreateEmbed {
    let mapper_url = beatmapset.mapper.replace(" ", "%20");
    let most_common_mode = {
        let modes: Vec<&Modes> = beatmapset
            .beatmaps
            .iter()
            .map(|beatmap| &beatmap.mode)
            .collect();

        match mode(&modes) {
            Some(mode) => mode,
            None => &Modes::Standard,
        }
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
                min_rating.unwrap(),
                max_rating.unwrap(),
                beatmapset.beatmaps.len()
            )
        }
    };

    let mut ranked_date_string: String = "**".to_string();
    if let Some(unix) = beatmapset.ranked_date_unix {
        ranked_date_string = format!(" <t:{}:R>**", unix)
    };

    let description = format!(
        "**[{}](https://osu.ppy.sh/beatmapsets/{})** | **{}{}\nMapped by [{}](https://osu.ppy.sh/users/{}) | [{}]\nArtist: {}\nSubmitted: <t:{}:R>\n\n{}",
        beatmapset.title, beatmapset.id, beatmapset.ranked_status, ranked_date_string, beatmapset.mapper, mapper_url, most_common_mode, beatmapset.artist, beatmapset.submitted_date_unix.unwrap(), star_rating_display_string
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
    let embed = CreateEmbed::new()
        .description(description)
        .colour(colour)
        .image(image);

    embed
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

fn parse_custom_button_id(s: &str) -> ButtonInteraction {
    let mut i = s.split(".").collect::<Vec<_>>().into_iter();
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

            let mut content: &str;

            while let Some(interaction) = interaction_stream.next().await {
                let parsed = parse_custom_button_id(&interaction.data.custom_id);
                match parsed.state {
                    ButtonState::Subscribe => {
                        match subscribe_to_beatmap(interaction.user.id.get() as i64, parsed.id)
                            .await
                        {
                            Ok(status) => match status {
                                database::mapfeed::UserAdditionStatus::UserAdded => {
                                    content = "Subscribed successfully"
                                }
                                database::mapfeed::UserAdditionStatus::UserAlreadyExists => {
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
                        match unsubscribe_from_beatmap(interaction.user.id.get() as i64, parsed.id)
                            .await
                        {
                            Ok(status) => match status {
                                database::mapfeed::UserDeletionStatus::UserRemoved => {
                                    content = "Unsubscribed successfully"
                                }
                                database::mapfeed::UserDeletionStatus::UserDoesNotExist => {
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

pub async fn populate() -> Result<(), Box<dyn std::error::Error>> {
    info!("Populating database");
    if let None = fetch_all_ids().await? {
        let ids = fetch_all_qualified_maps().await?;
        insert_beatmaps(ids).await?;
    };
    Ok(())
}

pub async fn subscription_handler(subscriber: i64, link: String) -> Result<(), SubscriptionError> {
    if OSU_LINK_REGEX.is_match(&link)? == false {
        return Err(SubscriptionError::InvalidLink);
    }
    debug!("{:?}", OSU_LINK_REGEX.captures(&link)?);
    let id = match OSU_LINK_REGEX.captures(&link)? {
        Some(capture) => match capture.get(1) {
            Some(id) => id.as_str().parse::<i32>()?,
            None => return Err(SubscriptionError::NonCapture),
        },
        None => return Err(SubscriptionError::InvalidLink),
    };

    subscribe_to_beatmap(subscriber, id).await?;
    Ok(())
}

pub async fn unsubscription_handler(
    subscriber: i64,
    link: String,
) -> Result<(), SubscriptionError> {
    if OSU_LINK_REGEX.is_match(&link)? == false {
        return Err(SubscriptionError::InvalidLink);
    }
    debug!("{:?}", OSU_LINK_REGEX.captures(&link)?);
    let id = match OSU_LINK_REGEX.captures(&link)? {
        Some(capture) => match capture.get(1) {
            Some(id) => id.as_str().parse::<i32>()?,
            None => return Err(SubscriptionError::NonCapture),
        },
        None => return Err(SubscriptionError::InvalidLink),
    };

    unsubscribe_from_beatmap(subscriber, id).await?;
    Ok(())
}

pub fn create_reply_with_sorted_beatmaps(beatmaps: Vec<Beatmapset>) -> CreateReply {
    let mut sorted_beatmaps = beatmaps;
    sorted_beatmaps.sort_by(|a, b| a.ranked_date_unix.cmp(&b.ranked_date_unix));

    CreateReply::default().ephemeral(true).embed(
        CreateEmbed::default()
            .title("Beatmaps you are subscribed to")
            .color(Colour::new(0x6758b8))
            .description(format!(
                "- {}",
                sorted_beatmaps
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
