mod playground;

use std::env;

use serenity::{
    model::channel::Message,
    prelude::*,
};
use dotenv::dotenv;

use playground::PlaygroundTask;

struct Handler;

impl EventHandler for Handler {
    fn message(&self, ctx: Context, message: Message) {
        let task = match PlaygroundTask::from_message(&message) {
            Some(task) => task,
            None => return
        };

        message.channel_id.say(&ctx.http, "Creating share link...").unwrap();
        message.channel_id.say(&ctx.http, task.create_share_link().unwrap()).unwrap();

        message.channel_id.say(&ctx.http, "Executing...").unwrap();

        let response = task.execute().unwrap();
        message.channel_id.say(&ctx.http, response.to_string()).unwrap();
    }
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
