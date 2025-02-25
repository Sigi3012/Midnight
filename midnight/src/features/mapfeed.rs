use crate::context::{Context, DiscordContextWrapper};
use anyhow::{Error, anyhow, bail};
use fancy_regex::Regex;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use midnight_database::{
    mapfeed::{
        delete_beatmap, fetch_all_subscribers_for_beatmap, fetch_all_tracked, insert_beatmaps,
    },
    subscriptions::{
        ChannelType, SubscriptionMode, beatmap_subscription_handler, fetch_all_subscribed_channels,
    },
};
use midnight_model::osu::{BeatmapStatus, Beatmapset, BeatmapsetVec, OsuGamemode};
use midnight_util::constants::{
    DISQUALIFIED_COLOUR, EMBED_BUTTON_TIMEOUT, ERROR_BACKOFF_COOLDOWN, LOVED_COLOUR,
    MAPFEED_LOOP_DURATION, QUALIFIED_COLOUR, RANKED_COLOUR,
};
use poise::{CreateReply, serenity_prelude as serenity};
use serenity::{
    all::{ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage},
    builder::{CreateEmbed, CreateEmbedFooter, CreateMessage},
    futures::StreamExt,
    model::{colour::Colour, id::ChannelId},
};
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::Hash,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    task,
    time::{Instant, sleep},
};

pub struct MapfeedManager;

struct MessageData {
    embed: CreateEmbed,
    subscribed_user_ids: Option<Vec<i64>>,
    beatmapset_data: Beatmapset,
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
                    sleep(ERROR_BACKOFF_COOLDOWN).await;
                } else {
                    sleep(MAPFEED_LOOP_DURATION).await;
                }
            }

            warn!("Mapfeed stopped")
        });

        Self
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

    let osu = Context::osu();
    let mut conn = Context::database().get_conn().await;

    info!("Fetching remote ids");
    let remote_ids = osu.fetch_all_qualified_maps().await?;

    info!("Fetching local ids");
    let local_ids: Vec<i32> = fetch_all_tracked(&mut conn).await?.unwrap_or_else(Vec::new);

    let remote_ids_hashset: HashSet<i32> = remote_ids.into_iter().collect();
    let local_ids_hashset: HashSet<i32> = local_ids.into_iter().collect();

    let new_maps: BeatmapsetVec = {
        let ids = remote_ids_hashset
            .difference(&local_ids_hashset)
            .cloned()
            .collect();

        osu.fetch_beatmaps(ids).await?
    };
    let changed_maps: BeatmapsetVec = {
        let ids = local_ids_hashset
            .difference(&remote_ids_hashset)
            .cloned()
            .collect();

        match osu.fetch_beatmaps(ids).await {
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

    let mut message_data = Vec::with_capacity(new_maps.len() * changed_maps.len());
    for map in new_maps {
        let embed = build_embed(&map);
        if let Err(why) = insert_beatmaps(&mut conn, vec![map.id]).await {
            error!(
                "ID: {} failed to insert into database, skipping insertion. Error: {}",
                map.id, why
            )
        } else {
            debug!("Inserted ID: {}", map.id)
        };

        message_data.push(MessageData {
            embed,
            subscribed_user_ids: None,
            beatmapset_data: map,
        })
    }
    for map in changed_maps {
        let embed = build_embed(&map);
        match fetch_all_subscribers_for_beatmap(&mut conn, map.id).await {
            Ok(subscribed_user_ids) => {
                clean_up_beatmap(map.id).await;
                message_data.push(MessageData {
                    embed,
                    subscribed_user_ids,
                    beatmapset_data: map,
                })
            }
            Err(e) => {
                error!(
                    "Failed to fetch subscribers for pk: {}, error: {}",
                    map.id, e
                )
            }
        }
    }

    match message_data.len() {
        x if x > 0 => info!("Sending {} unique messages", x),
        _ => {
            info!(
                "No messages to send. Mapfeed cycle took {:?} seconds",
                start_time.elapsed()
            );
        }
    }

    let subscribed_channels =
        fetch_all_subscribed_channels(&mut conn, ChannelType::Mapfeed(SubscriptionMode::Subscribe))
            .await?;
    debug!("Channel ids {:?}", subscribed_channels);

    for channel_id in subscribed_channels {
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

    info!("Mapfeed cycle took {:?} seconds", start_time.elapsed());
    Ok(())
}

pub fn build_embed(beatmapset: &Beatmapset) -> CreateEmbed {
    let mapper_url = beatmapset.mapper.replace(' ', "%20");
    let most_common_mode = {
        let modes: Vec<&OsuGamemode> = beatmapset
            .beatmaps
            .iter()
            .map(|beatmap| &beatmap.mode)
            .collect();

        mode(&modes).unwrap_or(&OsuGamemode::Osu)
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
        BeatmapStatus::Ranked => RANKED_COLOUR,
        BeatmapStatus::Qualified => QUALIFIED_COLOUR,
        BeatmapStatus::Loved => LOVED_COLOUR,
        BeatmapStatus::Pending | BeatmapStatus::Wip | BeatmapStatus::Graveyard => {
            DISQUALIFIED_COLOUR
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
    let mut conn = Context::database().get_conn().await;

    if let Err(why) = delete_beatmap(&mut conn, id).await {
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
    message_data: &MessageData,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Context::discord_ctx();

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
                .timeout(EMBED_BUTTON_TIMEOUT)
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

async fn handle_interaction(interaction: ComponentInteraction, ctx: &DiscordContextWrapper) {
    let mut conn = Context::database().get_conn().await;

    let content: &str;
    let parsed = parse_custom_button_id(&interaction.data.custom_id);
    match parsed.state {
        ButtonState::Subscribe => {
            match beatmap_subscription_handler(
                &mut conn,
                interaction.user.id.get() as i64,
                parsed.id,
                SubscriptionMode::Subscribe,
            )
            .await
            {
                Ok(_) => content = "Subscribed successfully",
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
            match beatmap_subscription_handler(
                &mut conn,
                interaction.user.id.get() as i64,
                parsed.id,
                SubscriptionMode::Unsubscribe,
            )
            .await
            {
                Ok(_) => content = "Unsubscribed successfully",
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
    let mut conn = Context::database().get_conn().await;
    info!("Populating database");
    if fetch_all_tracked(&mut conn).await?.is_none() {
        let ids = Context::osu().fetch_all_qualified_maps().await?;
        insert_beatmaps(&mut conn, ids).await?;
    };
    Ok(())
}

pub async fn subscription_handler(
    subscriber: i64,
    link: &str,
    mode: SubscriptionMode,
) -> anyhow::Result<()> {
    if !OSU_LINK_REGEX.is_match(link)? {
        bail!("Invalid link specified");
    }
    debug!("{:?}", OSU_LINK_REGEX.captures(link)?);
    let id = match OSU_LINK_REGEX.captures(link)? {
        Some(capture) => match capture.get(1) {
            Some(id) => id.as_str().parse::<i32>()?,
            None => bail!("Non capture"),
        },
        None => unreachable!(),
    };
    let mut conn = Context::database().get_conn().await;
    match mode {
        SubscriptionMode::Subscribe => {
            beatmap_subscription_handler(&mut conn, subscriber, id, SubscriptionMode::Subscribe)
                .await?;
        }
        SubscriptionMode::Unsubscribe => {
            beatmap_subscription_handler(&mut conn, subscriber, id, SubscriptionMode::Unsubscribe)
                .await?;
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

// TODO this is kinda more verbose than needed
fn mode<T: Eq + Hash + Clone>(input: &Vec<T>) -> Option<T> {
    let mut map = HashMap::new();
    for i in input {
        let count = map.entry(i).or_insert(0);
        *count += 1;
    }

    map.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(key, _)| key.clone())
}
