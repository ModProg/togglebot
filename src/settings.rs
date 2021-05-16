//! All configuration and state loading/saving logic.

#[cfg(test)]
use std::{collections::hash_map::DefaultHasher, hash::BuildHasherDefault};
use std::{io::ErrorKind, num::NonZeroU32};

use anyhow::Result;
use chrono::{Duration, prelude::*};
use derivative::Derivative;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{handler::AsyncState, Source, UserResponse};

#[cfg(not(test))]
type HashSet<T> = std::collections::HashSet<T>;
#[cfg(test)]
type HashSet<T> = std::collections::HashSet<T, BuildHasherDefault<DefaultHasher>>;
#[cfg(not(test))]
type HashMap<K, V> = std::collections::HashMap<K, V>;
#[cfg(test)]
type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<DefaultHasher>>;

#[derive(Derivative, Deserialize, Clone)]
#[derivative(Debug = "transparent", Default)]
pub struct Links(HashMap<String, String>);

type Commands = HashMap<String, CommandItem>;

#[derive(Derivative, Deserialize, Clone)]
#[derivative(Debug = "transparent")]
#[serde(untagged)]
pub enum CommandItem {
    Enabled(bool),
    Message(String),
    Custom(Command),
}

#[derive(Derivative, Deserialize, Clone)]
#[derivative(Debug)]
pub struct Command {
    args: Option<Vec<String>>,
    format: Option<String>,
    cooldown: Option<NonZeroU32>,
    pub aliases: Option<Vec<String>>,
}

impl Command {
    pub async fn respond(
        &self,
        name: &str,
        args: Option<&str>,
        state: AsyncState,
        source: Source,
    ) -> UserResponse {
        if let Some(cooldown) = self.cooldown {
            let mut state = state.write().await;
            if let Some(last_executed) = state.last_executed.get(name) {
                if *last_executed + Duration::seconds(u32::from(cooldown).into()) > Utc::now() {
                    return UserResponse::Unknown;
                }
            } 
            state.last_executed.insert(name.to_string(),Utc::now());
        }
        if let Some(format) = &self.format {
            info!("{}", format);
            UserResponse::Custom(format.clone())
        } else {
            UserResponse::Unknown
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub discord: Option<Discord>,
    pub twitch: Option<Twitch>,
    #[serde(default)]
    pub links: Links,
    pub commands: Commands,
}

#[derive(Deserialize, Derivative)]
#[derivative(Debug)]
pub struct Discord {
    #[derivative(Debug = "ignore")]
    pub token: String,
}

#[derive(Deserialize, Derivative)]
#[derivative(Debug)]
pub struct Twitch {
    pub login: String,
    #[derivative(Debug = "ignore")]
    pub token: String,
    pub channel: String,
}

impl IntoIterator for Links {
    type Item = (String, String);

    type IntoIter = std::collections::hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.clone().into_iter()
    }
}

pub async fn load_config() -> Result<Config> {
    let config = fs::read("/app/config.toml").await;
    let config = match config {
        Ok(c) => c,
        Err(_) => fs::read("config.toml").await?,
    };

    toml::from_slice(&config).map_err(Into::into)
}

#[derive(Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub schedule: BaseSchedule,
    #[serde(default)]
    pub off_days: HashSet<Weekday>,
    #[serde(default)]
    pub custom_commands: HashMap<String, HashMap<Source, String>>,
    pub last_executed: HashMap<String, DateTime<Utc>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schedule: BaseSchedule::default(),
            off_days: [Weekday::Sat, Weekday::Sun].iter().copied().collect(),
            custom_commands: HashMap::default(),
            last_executed: HashMap::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BaseSchedule {
    pub start: (NaiveTime, NaiveTime),
    pub finish: (NaiveTime, NaiveTime),
}

impl BaseSchedule {
    #[must_use]
    pub fn format_start(&self) -> String {
        Self::format_range(self.start)
    }

    #[must_use]
    pub fn format_finish(&self) -> String {
        Self::format_range(self.finish)
    }

    fn format_range(range: (NaiveTime, NaiveTime)) -> String {
        if range.0 == range.1 {
            range.0.format("%I:%M%P").to_string()
        } else {
            format!("{}~{}", range.0.format("%I:%M"), range.1.format("%I:%M%P"))
        }
    }
}

impl Default for BaseSchedule {
    fn default() -> Self {
        Self {
            start: (NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(8, 0, 0)),
            finish: (NaiveTime::from_hms(16, 0, 0), NaiveTime::from_hms(16, 0, 0)),
        }
    }
}

pub async fn load_state() -> Result<State> {
    let state = match fs::read("state.json").await {
        Ok(buf) => buf,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(State::default()),
        Err(e) => return Err(e.into()),
    };

    serde_json::from_slice(&state).map_err(Into::into)
}

pub async fn save_state(state: &State) -> Result<()> {
    let json = serde_json::to_vec_pretty(state)?;

    fs::write("~temp-state.json", &json).await?;
    fs::rename("~temp-state.json", "state.json").await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn ser_default() {
        let output = serde_json::to_value(&State::default()).unwrap();
        let expect = json! {{
            "schedule": {
                "start": [
                    "07:00:00",
                    "08:00:00"
                ],
                "finish": [
                    "16:00:00",
                    "16:00:00"
                ]
            },
            "off_days": ["Sat", "Sun"],
            "custom_commands": {}
        }};

        assert_eq!(expect, output);
    }

    #[test]
    fn ser_custom() {
        let output = serde_json::to_value(&State {
            schedule: BaseSchedule {
                start: (
                    NaiveTime::from_hms(5, 30, 0),
                    NaiveTime::from_hms(7, 20, 11),
                ),
                finish: (
                    NaiveTime::from_hms(16, 0, 0),
                    NaiveTime::from_hms(17, 15, 20),
                ),
            },
            off_days: [Weekday::Mon].iter().copied().collect(),
            custom_commands: vec![(
                "hello".to_owned(),
                vec![(Source::Discord, "Hello World!".to_owned())]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
            last_executed: HashMap::default(),
        })
        .unwrap();
        let expect = json! {{
            "schedule": {
                "start": [
                    "05:30:00",
                    "07:20:11"
                ],
                "finish": [
                    "16:00:00",
                    "17:15:20"
                ]
            },
            "off_days": ["Mon"],
            "custom_commands": {
                "hello": {
                    "Discord": "Hello World!"
                }
            }
        }};

        assert_eq!(expect, output);
    }
}
