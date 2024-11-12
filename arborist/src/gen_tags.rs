use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;

pub async fn gen_summary(model: String, prompt: String, system: String) -> anyhow::Result<String> {
    let ollama = Ollama::default();
    let res = ollama
        .generate(GenerationRequest::new(model, prompt).system(system))
        .await?;

    Ok(res.response)
}
