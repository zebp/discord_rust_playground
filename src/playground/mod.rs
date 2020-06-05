use std::fmt;

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::model::channel::Message;

#[derive(Debug, Serialize)]
pub enum RustChannel {
    #[serde(rename = "stable")]
    Stable,
    #[serde(rename = "beta")]
    Beta,
    #[serde(rename = "nightly")]
    Nightly,
}

impl fmt::Display for RustChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RustChannel::Stable => "stable",
                RustChannel::Beta => "beta",
                RustChannel::Nightly => "nightly",
            }
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExecutionResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Deserialize)]
struct ShareResponse {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaygroundTask {
    channel: RustChannel,
    mode: &'static str,
    edition: &'static str,
    crate_type: &'static str,
    tests: bool,
    code: String,
    backtrace: bool,
}

impl PlaygroundTask {
    pub fn from_message(message: &Message) -> Option<Self> {
        let regex = Regex::new("!compile\\s```rust\\n((.*|\\n)*)```").unwrap(); // TODO: Make this a constant

        let captures = regex.captures(&message.content)?;
        let code = captures.get(1)?.as_str();

        Some(Self::new(String::from(code), RustChannel::Stable)) // TODO: Make a way to specify the channel
    }

    fn new(code: String, channel: RustChannel) -> Self {
        let tests = code.contains("#[test]"); // TODO: Make this detection smarter
        let crate_type = if tests { "lib" } else { "bin" };

        PlaygroundTask {
            mode: "debug", // TODO: Maybe make a release option?
            edition: "2018",
            backtrace: false,
            crate_type,
            channel,
            code,
            tests,
        }
    }

    pub async fn execute(&self) -> Result<ExecutionResponse, reqwest::Error> {
        Client::new()
            .post("https://play.rust-lang.org/execute")
            .json(self)
            .send()
            .await?
            .json()
            .await
    }

    pub async fn create_share_link(&self) -> Result<String, reqwest::Error> {
        let share_response: ShareResponse = Client::new()
            .post("https://play.rust-lang.org/meta/gist/")
            .json(&json!({"code": self.code}))
            .send()
            .await?
            .json()
            .await?;

        Ok(format!(
            "https://play.rust-lang.org/?version={}&mode=debug&edition=2018&gist={}",
            self.channel, share_response.id
        ))
    }
}

mod tests {

    use super::*;

    #[tokio::test]
    async fn execute_bin() {
        let channel = RustChannel::Stable;
        let code = String::from("fn main() {\n\tprintln!(\"Hello, world!\");\n}");

        let task = PlaygroundTask::new(code, channel);
        let response = task.execute().await.unwrap();
    }

    #[tokio::test]
    async fn execute_test() {
        let channel = RustChannel::Stable;
        let code = String::from("#[test]\nfn it_works() {\n\tassert!(true)\n}");

        let task = PlaygroundTask::new(code, channel);
        let response = task.execute().await.unwrap();
    }

    #[tokio::test]
    async fn create_share_link() {
        let channel = RustChannel::Stable;
        let code = String::from("fn main() {\n\tprintln!(\"Hello, world!\");\n}");

        let task = PlaygroundTask::new(code, channel);
        let url = task.create_share_link().await.unwrap();
    }
}
