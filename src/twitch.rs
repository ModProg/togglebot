//! Twitch service connector that allows to receive commands from Twitch channels.

use anyhow::Result;
use log::{error, info};
use tokio::{select, sync::oneshot};
use twitch_irc::{
    login::StaticLoginCredentials,
    message::{PrivmsgMessage, ServerMessage},
    ClientConfig, TCPTransport, TwitchIRCClient,
};

use crate::{
    settings::{Links, Twitch},
    Message, Queue, Response, Shutdown, Source, UserResponse,
};

type Client = TwitchIRCClient<TCPTransport, StaticLoginCredentials>;

#[allow(clippy::missing_panics_doc)]
pub async fn start(config: &Twitch, queue: Queue, mut shutdown: Shutdown) -> Result<()> {
    let irc_config = ClientConfig::new_simple(StaticLoginCredentials::new(
        config.login.clone(),
        Some(config.token.clone()),
    ));
    let (mut messages, client) = Client::new(irc_config);
    let channel = config.channel.clone();

    client.join(channel.clone());

    tokio::spawn(async move {
        loop {
            select! {
                _ = shutdown.recv() => break,
                message = messages.recv() => {
                    if let Some(message) = message {
                        let client = client.clone();
                        let queue = queue.clone();
                        let channel = channel.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_server_message(queue, message, client, channel).await {
                                error!("error during event handling: {}", e);
                            }
                        });
                    } else {
                        break;
                    }
                }
            }
        }

        info!("twitch connection shutting down");
    });

    Ok(())
}

async fn handle_server_message(
    queue: Queue,
    message: ServerMessage,
    client: Client,
    channel: String,
) -> Result<()> {
    match message {
        ServerMessage::Privmsg(msg) => handle_message(queue, msg, client, channel).await?,
        ServerMessage::Join(_) => info!("twitch connection ready, listening for events"),
        _ => {}
    }

    Ok(())
}

async fn handle_message(
    queue: Queue,
    msg: PrivmsgMessage,
    client: Client,
    channel: String,
) -> Result<()> {
    let message = Message {
        source: Source::Twitch,
        content: msg.message_text.clone(),
        admin: false,
    };
    let (tx, rx) = oneshot::channel();

    if queue.send((message, tx)).await.is_ok() {
        if let Ok(resp) = rx.await {
            match resp {
                Response::User(user_resp) => {
                    handle_user_message(user_resp, msg, client, channel).await?
                }
                Response::Admin(_) => {}
            }
        }
    }

    Ok(())
}

#[allow(clippy::match_same_arms)]
async fn handle_user_message(
    resp: UserResponse,
    msg: PrivmsgMessage,
    client: Client,
    channel: String,
) -> Result<()> {
    match resp {
        UserResponse::Commands(res) => handle_commands(msg, client, channel, res).await,
        UserResponse::Links(links) => handle_links(msg, client, channel, links).await,
        UserResponse::Schedule {
            start,
            finish,
            off_days,
        } => handle_schedule(msg, client, channel, start, finish, off_days).await,
        UserResponse::Custom(content) => handle_custom(msg, client, channel, content).await,
        UserResponse::Unknown => Ok(()),
        UserResponse::WrongArgs => Ok(()),
    }
}

async fn handle_commands(
    msg: PrivmsgMessage,
    client: Client,
    channel: String,
    res: Result<Vec<String>>,
) -> Result<()> {
    let message = match res {
        Ok(names) => format!("Available commands: !{}", names.join(", !")),
        Err(e) => {
            error!("failed listing commands: {}", e);
            "Sorry, something went wrong fetching the list of commands".to_owned()
        }
    };

    client
        .say_in_response(channel, message, Some(msg.message_id))
        .await?;

    Ok(())
}

async fn handle_links(
    msg: PrivmsgMessage,
    client: Client,
    channel: String,
    links: Links,
) -> Result<()> {
    client
        .say_in_response(
            channel,
            links
                .into_iter()
                .enumerate()
                .fold(String::new(), |mut list, (i, (name, url))| {
                    if i > 0 {
                        list.push_str(" | ");
                    }

                    list.push_str(&name);
                    list.push_str(": ");
                    list.push_str(&url);
                    list
                }),
            Some(msg.message_id),
        )
        .await?;

    Ok(())
}

async fn handle_schedule(
    msg: PrivmsgMessage,
    client: Client,
    channel: String,
    start: String,
    finish: String,
    off_days: Vec<String>,
) -> Result<()> {
    let last_off_day = off_days.len() - 1;
    let days = format!(
        "Every day, except {}",
        off_days
            .into_iter()
            .enumerate()
            .fold(String::new(), |mut days, (i, day)| {
                if i == last_off_day {
                    days.push_str(" and ");
                } else if i > 0 {
                    days.push_str(", ");
                }

                days.push_str(&day);
                days
            })
    );
    let time = format!("Starting around {}, finishing around {}", start, finish);

    client
        .say_in_response(
            channel,
            format!("{} | {} | Timezone CET", days, time),
            Some(msg.message_id),
        )
        .await?;
    info!("Replied");

    Ok(())
}

async fn handle_custom(
    msg: PrivmsgMessage,
    client: Client,
    channel: String,
    content: String,
) -> Result<()> {
    client
        .say_in_response(channel, content, Some(msg.message_id))
        .await?;

    Ok(())
}
