use anyhow::Result;
use clap::Parser;

mod file_management;
mod gen_tags;

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
async fn main() -> Result<()> {
    let args = Args::parse();

    //let model = "qwen2.5:1.5b".to_string();
    //let prompt = "How many r's does strawberry have".to_string();
    //let system =
    //    "You are a helpful assistant who only replies with 2-3 sentences and not more.".to_string();
    //
    //let response = gen_tags::gen_summary(model, prompt, system).await?;
    //println!("{}", response);

    let _ = file_management::scan_dir(args.path);
    println!("hi");

    Ok(())
}
