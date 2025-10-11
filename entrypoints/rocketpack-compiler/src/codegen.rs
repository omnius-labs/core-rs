use crate::{config::AppConfig, error::CodegenError};

mod rust;

pub async fn generate(conf: AppConfig) -> Result<(), CodegenError> {
    for generator_conf in conf.generators {
        match generator_conf.plugin.as_str() {
            "rocketpack-rust" => rust::generate(&conf.sources, &generator_conf).await?,
            "rocketpack-csharp" => {}
            "rocketpack-swift" => {}
            _ => {}
        }
    }

    Ok(())
}
