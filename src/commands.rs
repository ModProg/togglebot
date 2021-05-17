use chrono::{Duration, Utc};
use dynfmt::{Format, SimpleCurlyFormat};
use log::info;
use reqwest::StatusCode;

use crate::{
    handler::AsyncState,
    settings::{Command, FormatString},
    Source, UserResponse,
};

#[derive(Clone, Debug)]
pub enum Type {
    String,
    Url(String),
}

impl Type {
    pub fn parse(s: &str, arg: &str) -> Self {
        match s {
            _ if s.eq_ignore_ascii_case("url") => Type::Url(arg.to_owned()),
            _ => Type::String,
        }
    }

    pub async fn format(&self, s: &str) -> Option<String> {
        if self.test(s).await {
            match self {
                Type::String => Some(s.to_owned()),
                Type::Url(format) => Some(
                    SimpleCurlyFormat
                        .format(format, &[s])
                        .unwrap()
                        .to_owned()
                        .to_string(),
                ),
            }
        } else {
            None
        }
    }
    pub async fn test(&self, s: &str) -> bool {
        match self {
            Type::String => true,
            Type::Url(format) => {
                let link = if let Ok(formatted) = SimpleCurlyFormat.format(format, &[s]) {
                    formatted.to_string()
                } else {
                    return false;
                };
                info!("Trying to reach: {}", link);
                let resp = match reqwest::Client::builder()
                    .user_agent("ToggleBot")
                    .build()
                    .expect("The client to be buildable")
                    .get(&link)
                    .send()
                    .await
                {
                    Ok(resp) => resp,
                    Err(_) => return false,
                };

                match resp.status() {
                    StatusCode::OK => return true,
                    _ => return false,
                }
            }
        }
    }
}

impl Command {
    pub async fn respond(
        &self,
        name: &str,
        args: Option<&str>,
        state: AsyncState,
        source: Source,
    ) -> UserResponse {
        if !self.platforms.contains(&source) {
            return UserResponse::Unknown;
        }
        if let Some(cooldown) = self.cooldown {
            let mut state = state.write().await;
            if let Some(last_executed) = state.last_executed.get(name) {
                if *last_executed + Duration::seconds(u32::from(cooldown).into()) > Utc::now() {
                    return UserResponse::Unknown;
                }
            }
            state.last_executed.insert(name.to_string(), Utc::now());
        }
        if let Some(format) = &self.format {
            let format = match format {
                FormatString::Universal(format) => format,
                FormatString::Specific(map) => {
                    if let Some(format) = map.get(&source) {
                        format
                    } else {
                        return UserResponse::Unknown;
                    }
                }
            };
            if let Some(wanted_args) = &self.args {
                if let Some(provided_args) =
                    args.map(|a| a.split_whitespace().collect::<Vec<&str>>())
                {
                    let mut args: Vec<String> = Vec::with_capacity(wanted_args.len());
                    for (t, v) in wanted_args.iter().zip(provided_args.iter()) {
                        match t {
                            crate::settings::Argument::Simple => args.push(v.to_string()),
                            crate::settings::Argument::Test(t) => {
                                if t.test(v).await {
                                    args.push(v.to_owned().to_string());
                                } else {
                                    return UserResponse::WrongArgs;
                                }
                            }
                            crate::settings::Argument::Format(t) => {
                                if let Some(formatted) = t.format(v).await {
                                    args.push(formatted);
                                } else {
                                    return UserResponse::WrongArgs;
                                }
                            }
                        }
                    }
                    if let Ok(formated) = SimpleCurlyFormat.format(&format, &args) {
                        return UserResponse::Custom(formated.to_string());
                    }
                }
                UserResponse::WrongArgs
            } else {
                UserResponse::Custom(format.clone())
            }
        } else {
            UserResponse::Unknown
        }
    }
}
