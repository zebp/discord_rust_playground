mod playground;

use std::env;

use dotenv::dotenv;
use serenity::{
    framework::{
        standard::{
            macros::{command, group, hook},
            Args, CommandResult,
        },
        StandardFramework,
    },
    model::{channel::Message, id::ChannelId},
    prelude::*,
};
use thiserror::Error;

use playground::{CrateType, PlaygroundTask, RustChannel};
use regex::Regex;

#[derive(Debug, Error)]
enum TaskMessageError {
    #[error("Http request error")]
    RequestError(#[from] reqwest::Error),
    #[error("Error while sending discord message")]
    DiscordError(#[from] serenity::Error),
    #[error("No code provided")]
    NoCode,
    #[error("No code provided")]
    InvalidCodeFormat,
    #[error("Provided rust channel does not exist, please use Stable, Beta, or Nightly")]
    InvalidRustChannel,
}

#[group]
#[commands(rust)]
struct General;

#[command]
async fn rust(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let first_arg = args.current().ok_or(TaskMessageError::NoCode)?;
    let channel = if first_arg.starts_with("```rust") {
        RustChannel::Stable
    } else {
        args.single()
            .map_err(|_| TaskMessageError::InvalidRustChannel)?
    };

    let regex = Regex::new("```rust\\n((.*|\\n)*)```").unwrap();
    let code = args.rest();

    let captures = regex
        .captures(code)
        .ok_or(TaskMessageError::InvalidCodeFormat)?;
    let code = captures
        .get(1)
        .ok_or(TaskMessageError::InvalidCodeFormat)?
        .as_str();
    let crate_type = if code.contains("fn main") {
        CrateType::Bin
    } else {
        CrateType::Lib
    };

    let playground_task = PlaygroundTask::new(code.into(), channel, crate_type);
    evaluate(playground_task, &msg.channel_id, &ctx).await?;

    Ok(())
}

async fn evaluate(
    task: PlaygroundTask,
    channel: &ChannelId,
    ctx: &Context,
) -> Result<(), TaskMessageError> {
    channel.say(&ctx.http, "Executing...").await?;

    let share_link = task.create_share_link().await?;
    let mut response = task.execute().await?;

    // Make sure the message is never too long
    response.stdout.truncate(900);
    response.stderr.truncate(900);

    channel
        .send_message(&ctx.http, |m| {
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
        })
        .await?;

    Ok(())
}

#[hook]
async fn after(ctx: &Context, msg: &Message, _: &str, command_result: CommandResult) {
    if let Err(error) = command_result {
        let _ = msg
            .channel_id
            .send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title("Rust Playground")
                        .description("Error occured while evaluating **Rust**.")
                        .color((255, 35, 35))
                        .field("Message", error.0, false)
                })
            })
            .await;
    }
}

#[tokio::main]
async fn main() {
    if let Err(_) = dotenv() {
        eprintln!("error loading env file, assuming DISCORD_TOKEN is already in env vars");
    }

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c.with_whitespace(true).prefix("!"))
        .group(&GENERAL_GROUP)
        .after(after);
    let mut client = Client::new(&token)
        .framework(framework)
        .await
        .expect("unable to create discord client");

    if let Err(error) = client.start().await {
        eprintln!("error occured when running bot: {:?}", error);
    }
}
