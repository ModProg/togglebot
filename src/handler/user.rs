use chrono::Weekday;
use log::info;

use super::AsyncState;
use crate::{Source, UserResponse, settings::{Command, CommandItem, Config}};

pub async fn commands(config: &Config, source: Source) -> UserResponse {
    info!("user: received `commands` command");
    UserResponse::Commands(Ok(list_command_names(config, source).await))
}

async fn list_command_names(config: &Config, source: Source) -> Vec<String> {
    config
        .commands
        .iter()
        .filter_map(|(name, ci)| match ci {
            CommandItem::Message(_) => Some(name.to_string()),
            CommandItem::Custom(Command {
                aliases: None,
                platforms,
                ..
            }) if platforms.contains(&source) => Some(name.to_string()),
            CommandItem::Custom(Command {
                aliases: Some(aliases),
                ..
            }) => match aliases.len() {
                0 => Some(name.to_string()),
                _ => Some(format!("{} (or !{})", name, aliases.join(", !"))),
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
                    aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
                } else {
                    false
                }
            }
        }) {
            match ci {
                CommandItem::Message(m) => UserResponse::Custom(m.clone()),
                CommandItem::Custom(c) => c.respond(cn, args, state, source).await,
                CommandItem::Function(..) => todo!()
            }
        } else {
            UserResponse::Unknown
        }
    } else {
        UserResponse::Unknown
    }
}
