use anyhow::Ok;
use clap::Parser;
use fastembed::{EmbeddingModel, InitOptions, SparseTextEmbedding, TextEmbedding};
use qdrant_client::qdrant::ScrollPointsBuilder;
use qdrant_client::Qdrant;
use summary::generate_file_summary;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use utils::DirScanConfig;

mod database;
mod file_management;
mod summary;
mod utils;

#[derive(Debug, clap::Parser)]
#[clap(
    name = "Arborist",
    about = "Arborist is a file-management utility tool powered by dark forces."
)]
struct Args {
    #[clap(short, long)]
    path: String,
    #[clap(short, long)]
    query: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let client = Qdrant::from_url("http://localhost:6334").build()?;
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::NomicEmbedTextV15).with_show_download_progress(true),
    )?;
    let sparse_model = SparseTextEmbedding::try_new(Default::default())?;

    database::create_hybrid_collection(&client, "file_data").await?;

    let scan_config = DirScanConfig::new(args.path);
    let mut scan_result = scan_config.scan_dir().await?;

    //println!("{:#?}", scan_result);
    let tokenizer = Tokenizer::from_pretrained("bert-base-cased", None).unwrap();
    let max_tokens = 20..40;
    let splitter = TextSplitter::new(ChunkConfig::new(max_tokens).with_sizer(tokenizer));

    //Generate summaries for each folder
    for file in &mut scan_result.file_metadata_list {
        println!("generating summary for: {:#?}", file);
        let summary = generate_file_summary("gemma2:2b", file).await?;
        println!("--------------------------------------------");
        println!("File Summary: {}", summary);
        println!("--------------------------------------------");
        file.summary = summary.clone();

        let chunks = splitter.chunks(&summary);
        for chunk in chunks {
            println!("chunk here: {}", chunk)
        }

        //let chunks = splitter.chunks(summary;
        let summary_vec = vec![summary];
        let embeddings = model.embed(summary_vec.clone(), None)?;
        let _sparse_embeddings = sparse_model.embed(summary_vec, None)?;

        println!("Embeddings length: {}", embeddings.len()); // -> Embeddings length: 4
        println!("Embedding dimension: {}", embeddings[0].len()); // -> Embedding dimension: 384
    }

    let query_vector = model.embed([args.query].to_vec(), None)?[0].clone(); // Example query vector
    let collection_name = "file_data";

    // Query and print file paths using dense vector matching
    database::query_and_print_file_paths(&client, collection_name, query_vector, 5, false).await?;

    //database::process_and_upload_files(&client, &scan_result.file_metadata_list).await?;

    let a = client
        .scroll(ScrollPointsBuilder::new("file_data").limit(10))
        .await?;

    //println!("All elems: {:#?}", a);

    Ok(())
}
