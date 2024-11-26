use clap::{Parser, Subcommand};
use log::{debug, info};
use qdrant_client::Qdrant;
use std::path::PathBuf;

use crate::config::Config;
use arborist::database::{self, chunk_string};
use arborist::summary::generate_file_summary;
use arborist::utils::{setup_fastembed, DirScanConfig};

mod config;

#[derive(Debug, clap::Parser)]
#[clap(
    name = "Arborist",
    about = "Arborist is a file-management utility tool powered by dark forces."
)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Scan {
        // file path to scan
        #[arg()]
        path: PathBuf,
    },

    Query {
        // query string from user
        #[arg()]
        query: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize env_logger
    env_logger::init();

    // Parse the Cli
    let cli = Cli::parse();

    // Load the configuration
    let config = Config::load(cli.config)?;
    info!("Loaded config: {:#?}", config);

    let (model, sparse_model) = setup_fastembed()?;

    // Initialize Qdrant client
    let client = Qdrant::from_url(&config.db_url).build()?;
    database::create_hybrid_collection(&client, &config.collection_name).await?;

    match &cli.command {
        Commands::Scan { path } => {
            let scan_config = DirScanConfig::new(path.to_path_buf());
            let mut scan_result = scan_config.scan_dir().await?;

            for file in &mut scan_result.file_metadata_list {
                let summary = generate_file_summary(&config.scan.model_name, file).await?;
                file.summary = summary.clone();
            }

            database::process_and_upload_files(&client, &scan_result.file_metadata_list).await?;
        }

        Commands::Query { query } => {
            //let transformed_query = chunk_string(query, "bert-base-cased", 20..40);
            let sparse_query_vector = sparse_model.embed([query].to_vec(), None)?;
            let query_vector = model.embed([query].to_vec(), None)?[0].clone();
            debug!("Query Vector: {:?}", query_vector);
            debug!("Sparse Query Vector: {:?}", sparse_query_vector);

            database::query_and_print_file_paths(
                &client,
                &config.collection_name,
                query_vector,
                config.query.top_k_results,
                false,
            )
            .await?;
        }
    }

    Ok(())
}
