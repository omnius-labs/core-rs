use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::{codegen::generate, config::ManifestGraph, error::CodegenError, semantic::SemanticGraph};

mod codegen;
mod config;
mod error;
mod parser;
mod semantic;

#[derive(Debug, Parser)]
#[command(author, version, about = "rocketpack format compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Compile {
        #[arg(value_name = "DIR", default_value = "./")]
        dir: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=off"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();

    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CodegenError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { dir } => run_compile(&dir).await?,
    }

    Ok(())
}

async fn run_compile(dir: &Path) -> Result<(), CodegenError> {
    let manifests = ManifestGraph::load(dir.join("rocketpack.yaml")).await?;
    let graph = SemanticGraph::build(manifests)?;
    generate(&graph).await?;
    Ok(())
}
