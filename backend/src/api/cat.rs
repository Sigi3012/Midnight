use crate::{api::CAT_API_SECRET, REQWEST_CLIENT};
use log::info;
use reqwest::header::{HeaderMap, HeaderName};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::{str::FromStr, vec};
use thiserror::Error;

const BASE_URL: &str = "https://api.thecatapi.com/v1/images/search";

lazy_static! {
    // Custom header types
    pub static ref X_API_KEY: HeaderName = HeaderName::from_str("x-api-key").unwrap();
    pub static ref ORDER: HeaderName = HeaderName::from_str("order").unwrap();

    pub static ref HEADERS: HeaderMap = {
        let mut headers = HeaderMap::new();
        headers.insert(X_API_KEY.clone(), CAT_API_SECRET.parse().unwrap());
        headers.insert(ORDER.clone(), "RAND".parse().unwrap());
        headers
    };
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO/FIXME move lines 27 to 70, to types.rs
#[derive(Debug, Error)]
pub enum Error {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Serde parse error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ResponseJson {
    #[serde(deserialize_with = "deserialize_breed")]
    pub breeds: Option<Breed>,
    pub url: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Breed {
    pub name: String,
}

impl ResponseJson {
    fn from_strings(responses: Vec<String>) -> Result<Vec<Self>> {
        let mut v: Vec<Self> = vec![];
        for s in responses {
            let mut deserialized: Vec<Self> = serde_json::from_str(&s)?;
            v.push(deserialized.pop().unwrap())
        }
        Ok(v)
    }
}

fn deserialize_breed<'de, D>(deserializer: D) -> std::result::Result<Option<Breed>, D::Error>
where
    D: Deserializer<'de>,
{
    let breeds: Vec<Value> = Deserialize::deserialize(deserializer)?;
    if breeds.is_empty() {
        Ok(None)
    } else {
        let breed: Breed =
            serde_json::from_value(breeds[0].clone()).map_err(serde::de::Error::custom)?;
        Ok(Some(breed))
    }
}

pub async fn get_random_image(count: i32) -> Result<Vec<ResponseJson>> {
    let client = REQWEST_CLIENT.clone();
    info!("Attempting to fetch {} images", count);

    let mut responses: Vec<String> = vec![];
    for _ in 0..count {
        let response = client.get(BASE_URL).headers(HEADERS.clone()).send().await?;

        responses.push(response.text().await?);
    }

    let result = ResponseJson::from_strings(responses)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;

    #[test]
    fn test_derserialization() {
        let mock: Vec<String> = vec![r#"
            [{"breeds":[{"weight":{"imperial":"7 - 15","metric":"3 - 7"},"id":"chau","name":"Chausie","temperament":"Affectionate, Intelligent, Playful, Social","origin":"Egypt","country_codes":"EG","country_code":"EG","description":"For those owners who desire a feline capable of evoking the great outdoors, the strikingly beautiful Chausie retains a bit of the wild in its appearance but has the house manners of our friendly, familiar moggies. Very playful, this cat needs a large amount of space to be able to fully embrace its hunting instincts.","life_span":"12 - 14","indoor":0,"alt_names":"Nile Cat","adaptability":5,"affection_level":5,"child_friendly":4,"dog_friendly":5,"energy_level":4,"grooming":3,"health_issues":1,"intelligence":5,"shedding_level":3,"social_needs":3,"stranger_friendly":4,"vocalisation":1,"experimental":1,"hairless":0,"natural":0,"rare":0,"rex":0,"suppressed_tail":0,"short_legs":0,"wikipedia_url":"https://en.wikipedia.org/wiki/Chausie","hypoallergenic":0,"reference_image_id":"vJ3lEYgXr"}],"id":"r0s90j0I8","url":"https://cdn2.thecatapi.com/images/r0s90j0I8.jpg","width":2093,"height":2105}]
        "#,
        r#"
        [{"breeds":[],"id":"2ls","url":"https://cdn2.thecatapi.com/images/2ls.jpg","width":500,"height":333}]
        "#].iter_mut().map(|s| s.to_string()).collect();
        let deserialized = ResponseJson::from_strings(mock);
        let expected_long = ResponseJson {
            breeds: Some(Breed {
                name: "Chausie".to_string(),
            }),
            url: "https://cdn2.thecatapi.com/images/r0s90j0I8.jpg".to_string(),
        };
        let expected_short = ResponseJson {
            breeds: None,
            url: "https://cdn2.thecatapi.com/images/2ls.jpg".to_string(),
        };

        let mut iter = deserialized.unwrap().into_iter();
        assert_eq!(expected_long, iter.next().unwrap());
        assert_eq!(expected_short, iter.next().unwrap())
    }

    #[test]
    fn test_headers() {
        dotenv().ok();
        assert!(!HEADERS.is_empty())
    }

    #[tokio::test]
    async fn test_requestor() {
        dotenv().ok();
        let res = get_random_image(3).await;
        assert!(res.is_ok());
        assert_eq!(3, res.unwrap().len());
    }
}
