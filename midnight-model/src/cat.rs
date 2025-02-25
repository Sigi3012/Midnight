use anyhow::Result;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

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
    pub fn from_strings(responses: Vec<String>) -> Result<Vec<Self>> {
        let mut v: Vec<Self> = vec![];
        for s in responses {
            let mut deserialized: Vec<Self> = serde_json::from_str(&s)?;
            v.append(&mut deserialized)
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
