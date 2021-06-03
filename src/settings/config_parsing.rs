#[cfg(test)]
use std::{collections::hash_map::DefaultHasher, hash::BuildHasherDefault};
use std::{env, num::NonZeroU32, str::FromStr};

use derivative::Derivative;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use serde_with::DeserializeFromStr;

use crate::{commands::Type, Source};

#[cfg(not(test))]
type HashMap<K, V> = std::collections::HashMap<K, V>;
#[cfg(test)]
type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<DefaultHasher>>;

#[derive(Derivative, Deserialize, Clone)]
#[derivative(Debug = "transparent", Default)]
pub struct Links(HashMap<String, String>);

type Commands = HashMap<String, CommandItem>;

#[derive(Debug, DeserializeFromStr, Clone)]
pub struct NamedFunction(String, String);

#[derive(Derivative, Deserialize, Clone)]
#[derivative(Debug = "transparent")]
#[serde(untagged)]
pub enum CommandItem {
    Function(NamedFunction),
    Message(String),
    Custom(Command),
}

fn all_platforms() -> Vec<Source> {
    vec![Source::Discord, Source::Twitch]
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FormatString {
    Universal(String),
    Specific(HashMap<Source, String>),
}

#[derive(DeserializeFromStr, Clone, Debug)]
pub enum Argument {
    Simple(Type),
    Test(Type),
    Format(Type),
}

#[derive(Derivative, Deserialize, Clone)]
#[derivative(Debug)]
pub struct Command {
    pub args: Option<Vec<Argument>>,
    pub format: Option<FormatString>,
    pub cooldown: Option<NonZeroU32>,
    pub aliases: Option<Vec<String>>,
    #[serde(default = "all_platforms")]
    pub platforms: Vec<Source>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Platforms {
    Standard {
        discord: Option<Discord>,
        twitch: Option<Twitch>,
    },
    Custom(HashMap<String, Platform>),
}

#[derive(Deserialize, Debug)]
pub struct ConfigDto {
    platforms: Platforms,
    #[serde(default)]
    pub links: Links,
    pub commands: Commands,
}

pub fn env_token() -> String {
    env::var("BOT_TWITCH_TOKEN").expect("TOKEN")
}

#[derive(Clone, Deserialize, Derivative)]
#[derivative(Debug)]
pub struct Discord {
    #[derivative(Debug = "ignore")]
    pub token: String,
}

#[derive(Clone, Deserialize, Derivative)]
#[derivative(Debug)]
pub struct Twitch {
    pub login: String,
    #[derivative(Debug = "ignore")]
    #[serde(default = "env_token")]
    pub token: String,
    pub channel: String,
}

impl FromStr for Argument {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r"(?x)^
                    (?P<type>[a-zA-Z].*?)
                    (?:
                     (?P<seperator>[?!])
                     (?P<parsearg>.*?)
                     (?:
                      (:?<!>)
                      (?P<errorarg>.*)
                     )?
                    )?
                    $"
            )
            .expect("I should be ablet to write valid Regex");
        }

        let captures = RE.captures(s).expect("Always matches at least a type");
        println!("{:?}", captures);
        let type_name = captures
            .name("type")
            .expect("Always matches the beginning of the string.")
            .as_str();
        let seperator = if let Some(seperator) = captures.name("seperator") {
            Some(seperator.as_str())
        } else {
            None
        };
        let parsearg = if let Some(parsearg) = captures.name("parsearg") {
            Some(parsearg.as_str())
        } else {
            None
        };
        let errorarg = if let Some(errorarg) = captures.name("errorarg") {
            Some(errorarg.as_str())
        } else {
            None
        };
        Ok(match (seperator, parsearg, errorarg) {
            (None, _, _) => Argument::Simple(Type::parse(type_name, None, None)),
            (Some("!"), a, e) => Argument::Format(Type::parse(type_name, a, e)),
            (Some("?"), a, e) => Argument::Test(Type::parse(type_name, a, e)),
            _ => unreachable!("You found the secret ending"),
        })
    }
}

impl FromStr for NamedFunction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("@") {
            let split = s.splitn(3, '/').collect::<Vec<&str>>();
            if split.len() == 2 {
                Ok(Self(split[0].to_owned(), split[1].to_owned()))
            } else {
                Err(format!(
                    "Function name should contain two path elements `{}`",
                    s
                ))
            }
        } else {
            Err(format!("Function name should start with @: `{}`", s))
        }
    }
}
