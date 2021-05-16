use anyhow::bail;
use chrono::Weekday;
use log::info;
use reqwest::StatusCode;

use super::AsyncState;
use crate::{
    settings::{Command, CommandItem, Config},
    Source, UserResponse,
};

pub async fn commands(config: &Config, source: Source) -> UserResponse {
    info!("user: received `commands` command");
    UserResponse::Commands(Ok(list_command_names(config, source).await))
}

async fn list_command_names(config: &Config, source: Source) -> Vec<String> {
    config
        .commands
        .iter()
        .filter_map(|(name, ci)| match ci {
            CommandItem::Enabled(true)
            | CommandItem::Custom(Command { aliases: None, .. })
            | CommandItem::Message(_) => Some(name.to_string()),
            CommandItem::Custom(Command {
                aliases: Some(aliases),
                ..
            }) => match aliases.len() {
                0 => Some(name.to_string()),
                _ => Some(format!("{} (or {})", name, aliases.join(", "))),
            },
            _ => None,
        })
        .collect()
}

pub fn links(config: &Config, source: Source) -> UserResponse {
    info!("user: received `links` command");
    UserResponse::Links(match source {
        Source::Discord => config.links.clone(),
        Source::Twitch => config.links.clone(),
    })
}

pub async fn schedule(state: AsyncState) -> UserResponse {
    info!("user: received `schedule` command");

    let state = state.read().await;

    UserResponse::Schedule {
        start: state.schedule.format_start(),
        finish: state.schedule.format_finish(),
        off_days: state
            .off_days
            .iter()
            .map(|weekday| {
                match weekday {
                    Weekday::Mon => "Monday",
                    Weekday::Tue => "Tuesday",
                    Weekday::Wed => "Wednesday",
                    Weekday::Thu => "Thursday",
                    Weekday::Fri => "Friday",
                    Weekday::Sat => "Saturday",
                    Weekday::Sun => "Sunday",
                }
                .to_owned()
            })
            .collect(),
    }
}

pub fn ban(target: &str) -> UserResponse {
    info!("user: received `ban` command");
    UserResponse::Ban(target.to_owned())
}

pub async fn crate_(name: &str) -> UserResponse {
    info!("user: received `crate` command");

    let res = async {
        let link = format!("https://lib.rs/crates/{}", name);
        let resp = reqwest::Client::builder()
            .user_agent("ToggleBot")
            .build()?
            .get(&link)
            .send()
            .await?;

        Ok(match resp.status() {
            StatusCode::OK => link,
            StatusCode::NOT_FOUND => format!("Crate `{}` doesn't exist", name),
            s => bail!("unexpected status code {:?}", s),
        })
    };

    UserResponse::Crate(res.await)
}

pub async fn custom(
    config: &Config,
    state: AsyncState,
    source: Source,
    name: &str,
    args: Option<&str>,
) -> UserResponse {
    if let Some(name) = name.strip_prefix('!') {
        info!("{:?}", args);
        if let Some((cn, ci)) = config.commands.iter().find(|(key, val)| {
            if name.eq_ignore_ascii_case(key) {
                true
            } else {
                if let CommandItem::Custom(Command {
                    aliases: Some(aliases),
                    ..
                }) = val
                {
                    aliases.iter().any(|a| a.eq_ignore_ascii_case(key))
                } else {
                    false
                }
            }
        }) {
            match ci {
                CommandItem::Enabled(_) => UserResponse::Unknown,
                CommandItem::Message(m) => UserResponse::Custom(m.clone()),
                CommandItem::Custom(c) => c.respond(cn, args, state, source).await,
            }
        } else {
            UserResponse::Unknown
        }
    } else {
        UserResponse::Unknown
    }
}
