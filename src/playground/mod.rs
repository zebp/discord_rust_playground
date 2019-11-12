use std::fmt;

use regex::Regex;
use serde_json::json;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize)]
struct ExecutionResponse {
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
pub struct PlaygroundTask {
    channel: RustChannel,
    mode: &'static str,
    edition: &'static str,
    #[serde(rename = "crateType")]
    crate_type: &'static str,
    tests: bool,
    code: String,
    backtrace: bool,
}

impl PlaygroundTask {
    pub fn from_message(message: &Message) -> Option<Self> {
        let regex = Regex::new("<@643827675894513696>\\s```rust\\n((.*|\\n)*)```").unwrap(); // TODO: Make this a constant

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
            crate_type: crate_type,
            backtrace: false,
            channel,
            code,
            tests,
        }
    }

    pub fn execute(&self) -> Result<(String, String), reqwest::Error> {
        let client = Client::new();

        let response = client
            .post("https://play.rust-lang.org/execute")
            .json(self)
            .send()?;

        let execution_response: ExecutionResponse = response.json()?;

        Ok((execution_response.stdout, execution_response.stderr))
    }

   pub fn create_share_link(&self) -> Result<String, reqwest::Error> {
        let client = Client::new();

        let share_response: ShareResponse = client
            .post("https://play.rust-lang.org/meta/gist/")
            .json(&json!({"code": self.code}))
            .send()?
            .json()?;

        Ok(format!(
            "https://play.rust-lang.org/?version={}&mode=debug&edition=2018&gist={}",
            self.channel, share_response.id
        ))
    }
}

mod tests {

    use super::*;

    #[test]
    fn execute_bin() {
        let channel = RustChannel::Stable;
        let code = String::from("fn main() {\n\tprintln!(\"Hello, world!\");\n}");

        let task = PlaygroundTask::new(code, channel);
        let (stdout, _stderr) = task.execute().unwrap();

        assert_eq!(stdout, "Hello, world!\n");
    }

    #[test]
    fn execute_test() {
        let channel = RustChannel::Stable;
        let code = String::from("#[test]\nfn it_works() {\n\tassert!(true)\n}");

        let task = PlaygroundTask::new(code, channel);
        let (stdout, _stderr) = task.execute().unwrap();

        assert_eq!(stdout, include_str!("remote_test_result.txt"));
    }

    #[test]
    fn create_share_link() {
        let channel = RustChannel::Stable;
        let code = String::from("fn main() {\n\tprintln!(\"Hello, world!\");\n}");

        let task = PlaygroundTask::new(code, channel);
        let url = task.create_share_link().unwrap();

        dbg!(url);
    }
}
