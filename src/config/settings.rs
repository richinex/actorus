use anyhow::Result;
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub llm: LLMConfig,
    pub agent: AgentConfig,
    pub validation: ValidationConfig,
    pub system: SystemConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub max_orchestration_steps: usize,
    pub max_sub_goals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub agent_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub auto_restart: bool,
    pub heartbeat_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub check_interval_ms: u64,
    pub channel_buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let config_env = env::var("CONFIG_ENV").unwrap_or_else(|_| "default".to_string());

        let config = Config::builder()
            .add_source(File::with_name(&format!("config/{}", config_env)).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        config.try_deserialize()
    }

    pub fn api_key() -> Result<String> {
        env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))
    }
}
