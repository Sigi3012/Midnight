use fancy_regex::Regex;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::fs;

lazy_static! {
    #[derive(Debug)]
    static ref BUILT_PATTERNS: Vec<(Regex, String)> = build_all().unwrap();
}

#[derive(Deserialize)]
struct PatternReplacement {
    pattern: String,
    replacement: String,
}

fn load_json_patterns() -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let file = fs::read_to_string("patterns.json")?;
    let deserialized: Vec<PatternReplacement> = serde_json::from_str(&file)?;

    let vec_tuples: Vec<(String, String)> = deserialized
        .into_iter()
        .map(|pr| (pr.pattern, pr.replacement))
        .collect();

    Ok(vec_tuples)
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

fn build_all() -> Result<Vec<(Regex, String)>, Box<dyn std::error::Error>> {
    let mut patterns: Vec<(Regex, String)> = Vec::new();

    match load_json_patterns() {
        Ok(result) => {
            // [(p, r), (p, r), ..]
            for (p, r) in result.iter() {
                let regex_pattern = build_regex(&p)?;
                let regex_replacement = r.clone();
                patterns.push((regex_pattern, regex_replacement))
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
    // Check if a message contains a link within patterns.json patterns
    for p in BUILT_PATTERNS.iter() {
        match p.0.is_match(&message.content) {
            Ok(forwarded) => {
                if forwarded == true {
                    result = p.0.replace_all(&result, &p.1).to_string();
                    debug!("{}", result)
                }
            }
            Err(e) => warn!("{}", e),
        }
    }

    if &result == &message.content {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::Once};

    static INIT: Once = Once::new();

    fn initalize() {
        INIT.call_once(|| {
            fs::copy("../patterns.json", "patterns.json").unwrap();
        })
    }

    #[test]
    fn test_build_regex() {
        assert!(build_regex(r"^foo$").is_ok());
        assert!(build_regex(r"[").is_err());
    }

    #[test]
    fn test_build_all() {
        initalize();
        assert!(build_all().is_ok());
    }

    #[test]
    fn test_parse_json() {
        initalize();
        assert!(load_json_patterns().is_ok());
    }
}
