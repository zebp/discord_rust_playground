use std::{fmt, str::FromStr};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

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

impl FromStr for RustChannel {
    // We don't really care for a good error type here as we're going to use our own later.
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            "nightly" => Ok(Self::Nightly),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExecutionResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl ExecutionResponse {
    /// Formats the stderr to have a cleaner output with less noise.
    pub fn get_formatted_stderr(&self, task: &PlaygroundTask) -> String {
        let compiled = !self
            .stderr
            .contains("error: aborting due to previous error");

        if !compiled {
            return self
                .stderr
                .split("error: aborting due to previous error")
                .next()
                .unwrap()
                .lines()
                .skip(1)
                .collect::<Vec<&str>>()
                .join("\n");
        }

        if let CrateType::Lib = task.crate_type {
            return self.stderr.clone();
        }

        self.stderr
            .clone()
            .lines()
            .skip(3)
            .collect::<Vec<&str>>()
            .join("\n")
    }
}

#[derive(Debug, Deserialize)]
struct ShareResponse {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CrateType {
    Lib,
    Bin,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaygroundTask {
    channel: RustChannel,
    mode: &'static str,
    edition: &'static str,
    pub crate_type: CrateType,
    tests: bool,
    code: String,
    backtrace: bool,
}

impl PlaygroundTask {
    pub fn new(code: String, channel: RustChannel, crate_type: CrateType) -> Self {
        PlaygroundTask {
            mode: "debug", // TODO: Maybe make a release option?
            edition: "2018",
            backtrace: false,
            tests: crate_type == CrateType::Lib,
            crate_type,
            channel,
            code,
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

    #[tokio::test]
    async fn execute_bin() {
        use super::*;

        let channel = RustChannel::Stable;
        let code = String::from("fn main() {\n\tprintln!(\"Hello, world!\");\n}");

        let task = PlaygroundTask::new(code, channel, CrateType::Bin);
        let response = task.execute().await.unwrap();

        assert!(response.stdout.contains("Hello, world!"));
    }

    #[tokio::test]
    async fn execute_test() {
        use super::*;

        let channel = RustChannel::Stable;
        let code = String::from("#[test]\nfn it_works() {\n\tassert!(true)\n}");

        let task = PlaygroundTask::new(code, channel, CrateType::Lib);
        let response = task.execute().await.unwrap();

        assert!(response.stdout.contains("1 passed"));
    }

    #[tokio::test]
    async fn create_share_link() {
        use super::*;

        let channel = RustChannel::Stable;
        let code = String::from("fn main() {\n\tprintln!(\"Hello, world!\");\n}");

        let task = PlaygroundTask::new(code, channel, CrateType::Bin);
        task.create_share_link().await.unwrap();
    }
}
