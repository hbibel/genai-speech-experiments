use std::{env, path::PathBuf, str::FromStr};

use anyhow::Context;

pub struct Config {
    pub openai_key: String,
    pub recording_file: Option<PathBuf>,
}

const ENV_PREFIX: &str = "JARVIS_CODE__";

pub fn from_env() -> anyhow::Result<Config> {
    let openai_key = get_env("OPENAI_KEY")?;
    let recording_file = get_opt_env("RECORDING_FILE")
        .map(|s| PathBuf::from_str(&s).context("Could not parse provided recording file path"))
        .map_or(Ok(None), |v| v.map(Some))?;

    Ok(Config {
        openai_key,
        recording_file,
    })
}

fn get_env(key: &str) -> anyhow::Result<String> {
    env::var(format!("{ENV_PREFIX}{key}")).context(format!(
        "environment variable {ENV_PREFIX}{key} is required"
    ))
}

fn get_opt_env(key: &str) -> Option<String> {
    env::var(format!("{ENV_PREFIX}{key}")).ok()
}
