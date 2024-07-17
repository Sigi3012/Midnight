use fancy_regex::Regex;
use log::{debug, error, info};
use serde::Deserialize;

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
