use crate::commands::Type;

use super::HashMap;
use super::config_parsing::{Argument, ConfigDto, Links};
pub struct Config {
    pub platforms: HashMap<String, Platform>
}

impl From<ConfigDto> for Config{
    fn from(_: ConfigDto) -> Self {
        todo!()
    }
}

impl IntoIterator for Links {
    type Item = (String, String);

    type IntoIter = std::collections::hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
        // self.0.clone().into_iter()
    }
}

impl Argument {
    pub fn get_type(&self) -> &Type {
        match &self {
            Argument::Simple(t) | Argument::Test(t) | Argument::Format(t) => t,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Platform {
    Discord(Discord),
    Twitch(Twitch),
}

