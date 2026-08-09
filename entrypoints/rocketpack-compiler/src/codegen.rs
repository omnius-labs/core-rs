use tracing::{info, warn};

use crate::{error::CodegenError, semantic::SemanticGraph};

mod rust;

pub async fn generate(graph: &SemanticGraph) -> Result<(), CodegenError> {
    for generator_conf in &graph.root_manifest().config.generators {
        match generator_conf.plugin.as_str() {
            "rocketpack-rust" => rust::generate(graph, generator_conf).await?,
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
