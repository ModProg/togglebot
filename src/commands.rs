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
    String(usize),
    Url(String, Option<String>),
}

impl Type {
    pub fn parse(s: &str, arg: Option<&str>, earg: Option<&str>) -> Self {
        match s {
            "url" => Type::Url(
                arg.expect("Did not find an argument for url").to_owned(),
                earg.map(|s| s.to_owned()),
            ),
            _ if s.starts_with("string") => {
                Type::String(s.chars().filter(|c| '.' == *c).count().max(1))
            }
            _ => todo!("Hehe"),
        }
    }

    pub fn wanted_args(&self) -> usize {
        match self {
            Type::String(s) => *s,
            _ => 1,
        }
    }

    pub async fn format(&self, s: &str) -> Option<String> {
        if self.test(s).await {
            match self {
                Type::String(_) => Some(s.to_owned()),
                Type::Url(format, _) => Some(
                    SimpleCurlyFormat
                        .format(format, &[s])
                        .unwrap()
                        .to_owned()
                        .to_string(),
                ),
            }
        } else {
            match self {
                Type::Url(_, Some(format)) => Some(
                    SimpleCurlyFormat
                        .format(format, &[s])
                        .unwrap()
                        .to_owned()
                        .to_string(),
                ),
                _ => None,
            }
        }
    }
    pub async fn test(&self, s: &str) -> bool {
        match self {
            Type::String(_) => true,
            Type::Url(format, _) => {
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
                    let mut provided_args = provided_args.iter();
                    for t in wanted_args.iter() {
                        let argcount = t.get_type().wanted_args();
                        let combined_args = provided_args
                            .by_ref()
                            .take(argcount)
                            .map(|s| s.to_owned())
                            .collect::<Vec<&str>>()
                            .join(" ");
                        match t {
                            crate::settings::Argument::Simple(_) => args.push(combined_args),
                            crate::settings::Argument::Test(t) => {
                                if t.test(&combined_args).await {
                                    args.push(combined_args);
                                } else {
                                    return UserResponse::WrongArgs;
                                }
                            }
                            crate::settings::Argument::Format(t) => {
                                if let Some(formatted) = t.format(&combined_args).await {
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
