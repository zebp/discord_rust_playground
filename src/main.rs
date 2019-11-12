mod playground;

use std::env;

use serenity::{
    model::{channel::Message, id::ChannelId},
    prelude::*,
};
use dotenv::dotenv;
use thiserror::Error;

use playground::PlaygroundTask;

struct Handler;

impl EventHandler for Handler {
    fn message(&self, ctx: Context, message: Message) {
        let task = match PlaygroundTask::from_message(&message) {
            Some(task) => task,
            None => return
        };

        if let Err(error) = send_task_messages(task, &message.channel_id, &ctx) {
            message.channel_id.say(&ctx.http, "Error evaluating code.");
        }
    }
}

#[derive(Debug, Error)]
enum TaskMessageError {
    #[error("http request error")]
    RequestError(#[from] reqwest::Error),
    #[error("discord message")]
    DiscordError(#[from] serenity::Error)
}

fn send_task_messages(task: PlaygroundTask, channel: &ChannelId, ctx: &Context) -> Result<(), TaskMessageError> {
    channel.say(&ctx.http, "Creating share link...")?;
    channel.say(&ctx.http, task.create_share_link()?)?;

    channel.say(&ctx.http, "Executing...")?;
    let response = task.execute()?;
    channel.say(&ctx.http, response.to_string())?;

    Ok(())
}

fn main() {
    if let Err(_) = dotenv() {
        eprintln!("error loading env file, assuming DISCORD_TOKEN is already in env vars");
    }

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let mut client = Client::new(token, Handler).expect("unable to create discord client");

    if let Err(error) = client.start() {
        eprintln!("error occured when running bot: {:?}", error);
    }
}
