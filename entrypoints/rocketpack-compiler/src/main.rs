use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::{codegen::generate, config::AppConfig, error::CodegenError};

mod codegen;
mod config;
mod error;
mod parser;

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

    if let Err(_err) = run().await {
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
    let conf = AppConfig::load(dir.join("rocketpack.yaml")).await?;

    if let Err(e) = generate(conf).await {
        match e {
            CodegenError::Unexpected(_) => todo!(),
            CodegenError::Parse(parse_errors) => todo!(),
            CodegenError::Config(config_error) => todo!(),
            CodegenError::Other(_) => todo!(),
        }
    }

    todo!()
}

// let input_path = cli.input;
// let source = match fs::read_to_string(&input_path) {
//     Ok(contents) => contents,
//     Err(err) => {
//         eprintln!("読み込み失敗: {}: {}", input_path.display(), err);
//         return Err(1);
//     }
// };

// match generate(&source) {
//     Ok(code) => {
//         if let Some(path) = cli.output.as_deref() {
//             if let Some(parent) = path.parent() {
//                 if let Err(err) = fs::create_dir_all(parent) {
//                     eprintln!("ディレクトリ作成失敗: {}: {}", parent.display(), err);
//                     return Err(1);
//                 }
//             }
//             if let Err(err) = fs::write(path, code) {
//                 eprintln!("書き込み失敗: {}: {}", path.display(), err);
//                 return Err(1);
//             }
//         } else {
//             print!("{code}");
//         }
//         Ok(())
//     }
//     Err(errors) => {
//         display_parse_errors(input_path.display().to_string(), &source, &errors);
//         Err(1)
//     }
// }
//}
