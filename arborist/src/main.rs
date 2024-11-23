use clap::Parser;
use summary::generate_file_summary;
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let scan_config = DirScanConfig::new(args.path);
    let mut scan_result = scan_config.scan_dir().await?;

    //println!("{:#?}", scan_result);

    // Generate summaries for each folder
    for folder in &mut scan_result.file_metadata_list {
        println!("generating summary for: {:#?}", folder);
        let summary = generate_file_summary("gemma2:2b", folder).await?;
        println!("--------------------------------------------");
        println!("File Summary: {}", summary);
        println!("--------------------------------------------");
        folder.summary = summary;
    }

    Ok(())
}
