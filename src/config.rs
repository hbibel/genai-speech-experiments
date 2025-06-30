use std::env;

use anyhow::Context;

pub struct Config {
    pub openai_key: String,
}

const ENV_PREFIX: &str = "JARVIS_CODE__";

pub fn from_env() -> anyhow::Result<Config> {
    let openai_key = get_env("OPENAI_KEY")?;

    Ok(Config { openai_key })
}

fn get_env(key: &str) -> anyhow::Result<String> {
    env::var(format!("{ENV_PREFIX}{key}")).context(format!(
        "environment variable {ENV_PREFIX}{key} is required"
    ))
}
