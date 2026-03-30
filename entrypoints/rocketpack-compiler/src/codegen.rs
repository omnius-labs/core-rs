use tracing::{info, warn};

use crate::{config::AppConfig, error::CodegenError};

mod rust;

pub async fn generate(conf: AppConfig) -> Result<(), CodegenError> {
    for generator_conf in &conf.generators {
        match generator_conf.plugin.as_str() {
            "rocketpack-rust" => rust::generate(&conf.root_dir, &conf.sources, generator_conf).await?,
            "rocketpack-csharp" | "rocketpack-swift" => {
                info!(
                    generator_id = %generator_conf.id,
                    plugin = %generator_conf.plugin,
                    "skip non-rust generator for current rust-only implementation"
                );
            }
            _ => {
                warn!(generator_id = %generator_conf.id, plugin = %generator_conf.plugin, "skip unknown generator plugin");
            }
        }
    }

    Ok(())
}
