mod playground;

use std::env;

use dotenv::dotenv;
use serenity::{
    model::{channel::Message, id::ChannelId},
    prelude::*,
};
use thiserror::Error;

use playground::PlaygroundTask;

struct Handler;

impl EventHandler for Handler {
    fn message(&self, ctx: Context, message: Message) {
        let task = match PlaygroundTask::from_message(&message) {
            Some(task) => task,
            None => return,
        };

        if let Err(_) = send_task_messages(task, &message.channel_id, &ctx) {
            message
                .channel_id
                .say(&ctx.http, "Error evaluating code.")
                .expect("could not send an error message to discord");
        }
    }
}

#[derive(Debug, Error)]
enum TaskMessageError {
    #[error("http request error")]
    RequestError(#[from] reqwest::Error),
    #[error("discord message")]
    DiscordError(#[from] serenity::Error),
}

fn send_task_messages(
    task: PlaygroundTask,
    channel: &ChannelId,
    ctx: &Context,
) -> Result<(), TaskMessageError> {
    channel.say(&ctx.http, "Executing...")?;

    let share_link = task.create_share_link()?;
    let mut response = task.execute()?;

    // Make sure the message is never too long
    response.stdout.truncate(900);
    response.stderr.truncate(900);

    channel.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title("Rust Playground")
                .description(format!(
                    "Here is the code on the [Rust playground]({}).",
                    share_link
                ))
                .color((222, 165, 132));

            if !response.stdout.is_empty() {
                e.field("Stdout", format!("```\n{}```", response.stdout), false);
            }

            e.field("Stderr", format!("```\n{}```", response.stderr), false)
        })
    })?;

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
