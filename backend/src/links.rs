use ::serenity::all::{
    model::channel::MessageFlags, ChannelId, CreateInteractionResponseMessage, CreateMessage,
    EditMessage,
};
use common::context::get_context_wrapper;
use fancy_regex::Regex;
use futures::StreamExt;
use log::{debug, error, info, warn};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use tokio::time::Duration;

const BUTTON_TIMEOUT: Duration = Duration::from_secs(60);

lazy_static! {
    #[derive(Debug)]
    static ref BUILT_PATTERNS: Vec<BuiltPattern> = build_all().unwrap();
}

#[derive(Deserialize)]
struct LoadedJson {
    pattern: String,
    replacement: String,
}

#[derive(Debug)]
struct BuiltPattern {
    pattern: Regex,
    replacement: String,
}

fn load_json_patterns() -> Result<Vec<LoadedJson>, Box<dyn std::error::Error>> {
    let file = include_str!("../../patterns.json");
    let deserialized: Vec<LoadedJson> = serde_json::from_str(&file)?;

    Ok(deserialized)
}

fn build_regex(pattern: &str) -> Result<Regex, fancy_regex::Error> {
    match Regex::new(pattern) {
        Ok(regex) => return Ok(regex),
        Err(e) => {
            error!("Failed to compile regex pattern '{}': {}", pattern, e);
            return Err(e);
        }
    }
}

fn build_all() -> Result<Vec<BuiltPattern>, Box<dyn std::error::Error>> {
    let mut patterns: Vec<BuiltPattern> = Vec::new();

    match load_json_patterns() {
        Ok(jsons) => {
            for item in jsons.iter() {
                let regex_pattern = build_regex(&item.pattern)?;
                patterns.push(BuiltPattern {
                    pattern: regex_pattern,
                    replacement: item.replacement.to_owned(),
                });
            }
        }
        Err(e) => {
            error!("Failed to load json file, error: {}", e);
            return Err(e);
        }
    };

    info!("Built {} patterns successfully", patterns.len());

    Ok(patterns)
}

pub async fn fix_links(
    message: &poise::serenity_prelude::Message,
) -> Result<Option<String>, fancy_regex::Error> {
    let mut result = message.content.clone();
    // Check if a message contains a link within the loaded patterns
    for built in BUILT_PATTERNS.iter() {
        if built.pattern.is_match(&message.content)? {
            result = built
                .pattern
                .replace_all(&result, &built.replacement)
                .to_string();
            debug!("{}", result)
        }
    }

    if &result == &message.content {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

pub async fn message_handler(
    message_content: String,
    message_owner: u64,
    channel_target: ChannelId,
    reply_target: &serenity::Message,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = get_context_wrapper();
    let components = serenity::CreateActionRow::Buttons(vec![serenity::CreateButton::new(
        format!("{}", message_owner),
    )
    .emoji(serenity::ReactionType::Unicode("\u{1F5D1}".to_string()))
    .style(serenity::ButtonStyle::Danger)]);
    let builder = CreateMessage::new()
        .content(format!("<@{}>: {}", message_owner, message_content))
        .components(vec![components])
        .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
        .reference_message(reply_target);

    let mut message = channel_target.send_message(ctx, builder).await?;

    tokio::spawn(async move {
        let mut interaction_stream = message
            .await_component_interactions(&ctx.shard)
            .timeout(BUTTON_TIMEOUT)
            .stream();

        // Becomes none at the end of the timeout and continues
        while let Some(interaction) = interaction_stream.next().await {
            if interaction.user.id.get() == interaction.data.custom_id.parse::<u64>().unwrap() {
                if let Err(why) = interaction.message.delete(&ctx).await {
                    error!("Failed to delete message from interaction, {}", why)
                };
            } else {
                warn!("{} cannot press this button", interaction.user.name);
                match interaction
                    .create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content("You are not the owner of this message!")
                                .ephemeral(true),
                        ),
                    )
                    .await
                {
                    Ok(_) => (),
                    Err(e) => error!("Interaction failure, {}", e),
                }
            };

            match interaction
                .create_response(&ctx, serenity::CreateInteractionResponse::Acknowledge)
                .await
            {
                Ok(_) => (),
                Err(e) => error!("Interaction failure, {}", e),
            }
        }

        // FIXME define a new error please and thankyou
        if let Err(why) = message
            .edit(&ctx, EditMessage::new().components(vec![]))
            .await
        {
            error!("Failed to remove components, {}", why)
        };
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use poise::serenity_prelude::Message;

    trait Content {
        fn set_content(&mut self, content: impl Into<String>) -> &mut Self;
    }

    impl Content for Message {
        fn set_content(&mut self, content: impl Into<String>) -> &mut Self {
            self.content = content.into();
            self
        }
    }

    fn setup_test_message(content: impl Into<String>) -> Message {
        let mut message = Message::default();
        message.set_content(content);
        message
    }

    #[test]
    fn test_load_json() {
        let json = load_json_patterns();
        assert!(json.is_ok())
    }

    #[test]
    fn test_build_patterns() {
        let json = load_json_patterns().unwrap();
        let built = build_all();
        assert!(built.is_ok());
        assert_eq!(built.unwrap().len(), json.len())
    }

    #[tokio::test]
    async fn test_fix_singular_link() {
        let test_message = setup_test_message(
            "this is a test message, https://x.com/testaccount/status/1814183041708990884",
        );
        let result = fix_links(&test_message).await;
        assert!(result.is_ok());
        assert_eq!(
            &result.unwrap().unwrap(),
            "this is a test message, [Twitter](https://fxtwitter.com/testaccount/status/1814183041708990884)"
        )
    }

    #[tokio::test]
    async fn test_multiple_links() {
        let test_message = setup_test_message("https://x.com/testaccount/status/1814183041708990884, https://twitter.com/testaccount/status/1814183041708990884");
        let result = fix_links(&test_message).await;
        assert!(result.is_ok());
        assert_eq!(
            &result.unwrap().unwrap(),
            "[Twitter](https://fxtwitter.com/testaccount/status/1814183041708990884), [Twitter](https://fxtwitter.com/testaccount/status/1814183041708990884)"
        )
    }

    #[tokio::test]
    async fn test_twitter_link() {
        let test_message =
            setup_test_message("https://x.com/testaccount/status/1814183041708990884");
        let result = fix_links(&test_message).await;
        assert!(result.is_ok());
        assert_eq!(
            &result.unwrap().unwrap(),
            "[Twitter](https://fxtwitter.com/testaccount/status/1814183041708990884)"
        );
    }

    #[tokio::test]
    async fn test_instagram_link() {
        let test_message = setup_test_message("https://instagram.com/reel/foobar/?igsh=baz");
        let result = fix_links(&test_message).await;
        assert!(result.is_ok());
        assert_eq!(
            &result.unwrap().unwrap(),
            "[Instagram](https://ddinstagram.com/reel/foobar)"
        );
    }

    #[tokio::test]
    async fn test_tiktok_link() {
        let test_message = setup_test_message("https://vm.tiktok.com/foobar");
        let result = fix_links(&test_message).await;
        assert!(result.is_ok());
        assert_eq!(
            &result.unwrap().unwrap(),
            "[TikTok](https://vm.vxtiktok.com/foobar)"
        );
    }

    #[tokio::test]
    async fn test_pixiv_link() {
        let test_message = setup_test_message("https://www.pixiv.net/en/artworks/117847824");
        let result = fix_links(&test_message).await;
        assert!(result.is_ok());
        assert_eq!(
            &result.unwrap().unwrap(),
            "[Pixiv](https://phixiv.net/artworks/117847824)"
        );
    }

    #[tokio::test]
    async fn test_reddit_link() {
        let test_message = setup_test_message(
            "https://www.reddit.com/r/testcommunity/comments/something/somethingAgain/",
        );
        let result = fix_links(&test_message).await;
        assert!(result.is_ok());
        assert_eq!(
            &result.unwrap().unwrap(),
            "[Reddit](https://rxddit.com/r/testcommunity/comments/something/somethingAgain/)"
        );
    }
}
