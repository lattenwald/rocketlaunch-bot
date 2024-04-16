use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bot: BotConfig,
}

#[derive(Debug, Clone, Deserialize, Parser)]
pub struct BotConfig {
    #[arg(long, value_parser)]
    pub token: String,

    #[arg(long, value_parser)]
    pub admin_chats: Vec<i64>,
}

impl Config {
    pub fn parse_file(path: PathBuf) -> Self {
        let yaml_content = std::fs::read_to_string(path).expect("Failed to read config file");
        serde_yaml::from_str(&yaml_content).expect("Failed parsing config")
    }
}
