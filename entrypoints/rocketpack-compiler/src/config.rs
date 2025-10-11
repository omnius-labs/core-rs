use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml_ng::{Mapping, from_str};

use crate::error::ConfigError;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub generators: Vec<GeneratorConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SourceConfig {
    #[serde(rename = "base_dir")]
    pub base_dir: String,
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub excludes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratorConfig {
    pub id: String,
    pub plugin: String,
    #[serde(default)]
    pub options: Option<Mapping>,
    #[serde(default)]
    pub targets: Vec<GeneratorTargetConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratorTargetConfig {
    pub pattern: String,
    #[serde(default)]
    pub options: Option<Mapping>,
}

impl AppConfig {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_buf: PathBuf = path.as_ref().into();
        let contents = tokio::fs::read_to_string(&path_buf).await?;
        Self::from_yaml(&contents)
    }

    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        Ok(from_str(yaml)?)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn config_load_test() -> TestResult {
        let config_path = Path::new("./data/rocketpack.yaml");
        let config = AppConfig::load(config_path).await?;
        println!("{:?}", config);

        Ok(())
    }
}
