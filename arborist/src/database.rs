use crate::config::Config;
use crate::file_management::FileMetadata;
use crate::summary::generate_file_summary;
use crate::utils::setup_fastembed;
use anyhow::{Context, Result};
use fastembed::{SparseEmbedding, SparseTextEmbedding, TextEmbedding};
use log::info;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, Distance, Filter, PointStruct, QueryPointsBuilder,
    SearchParamsBuilder, SparseVectorParamsBuilder, SparseVectorsConfigBuilder, UpsertPoints,
    Value, VectorParamsBuilder, VectorsConfigBuilder,
};
use qdrant_client::{Payload, Qdrant};
use std::collections::HashMap;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use uuid::Uuid;

pub async fn create_hybrid_collection(client: &Qdrant, collection_name: &str) -> Result<()> {
    // Check if the collection already exists
    if client.collection_exists(collection_name).await? {
        info!(
            "Collection '{}' already exists. Skipping creation.",
            collection_name
        );
        return Ok(());
    }
    // Configure sparse vectors using builder
    let mut sparse_vector_config = SparseVectorsConfigBuilder::default();
    sparse_vector_config.add_named_vector_params("splade", SparseVectorParamsBuilder::default());

    // Configure dense vectors using builder
    let mut dense_vector_config = VectorsConfigBuilder::default();
    dense_vector_config
        .add_named_vector_params("novum", VectorParamsBuilder::new(768, Distance::Cosine));

    // Create collection using builders
    client
        .create_collection(
            CreateCollectionBuilder::new(collection_name)
                .sparse_vectors_config(sparse_vector_config)
                .vectors_config(dense_vector_config),
        )
        .await?;

    println!("New collection created");

    Ok(())
}

pub fn chunk_string(
    input: &str,
    tokenizer_name: &str,
    max_tokens: std::ops::Range<usize>,
) -> Vec<String> {
    // Initialize the tokenizer
    let tokenizer =
        Tokenizer::from_pretrained(tokenizer_name, None).expect("Failed to load tokenizer");

    // Create the TextSplitter with ChunkConfig
    let splitter = TextSplitter::new(ChunkConfig::new(max_tokens).with_sizer(tokenizer));

    // Chunk the input string and collect the results
    splitter
        .chunks(input)
        //.into_iter()
        .map(|chunk| chunk.to_string())
        .collect()
}

/// Generate both sparse and dense embeddings for a list of summaries
async fn generate_embeddings(
    summary: String,
    model: &TextEmbedding,
    sparse_model: &SparseTextEmbedding,
) -> Result<(Vec<Vec<f32>>, Vec<SparseEmbedding>)> {
    // Generate dense embeddings
    let summary_chunks = chunk_string(&summary, "bert-base-cased", 20..40);
    let dense_embeddings = model.embed(summary_chunks, None)?;

    // Generate sparse embeddings
    let sparse_embeddings = sparse_model.embed([summary].to_vec(), None)?;

    Ok((dense_embeddings, sparse_embeddings))
}

/// Checks if a file has already been indexed in the database
async fn is_file_already_indexed(client: &Qdrant, file_path: &str) -> anyhow::Result<bool> {
    let query_result = client
        .query(
            QueryPointsBuilder::new("file_data")
                .filter(Filter::must([Condition::matches(
                    "file_path",
                    file_path.to_string(),
                )]))
                .limit(1),
        )
        .await
        .context("Failed to query existing file")?;

    Ok(!query_result.result.is_empty())
}

/// Generate summary only if not already present
async fn get_or_generate_summary(
    config: &Config,
    file: &FileMetadata,
    force_regenerate: bool,
) -> Result<String> {
    // If summary is already present and we're not forcing regeneration, return it
    if !force_regenerate && !file.summary.is_empty() {
        return Ok(file.summary.clone());
    }

    // Generate summary
    generate_file_summary(&config.scan.model_name, file)
        .await
        .context("Failed to generate file summary")
}

/// Process and prepare files sequentially
pub async fn process_and_upload_files(
    client: &Qdrant,
    config: &Config,
    file_metadata_list: &[FileMetadata],
    force_regenerate: Option<bool>, // Changed to Option<bool>
) -> Result<()> {
    // Set default value if force_regenerate is None
    let force_regenerate = force_regenerate.unwrap_or(false);

    // Setup embedding models
    let (model, sparse_model) = setup_fastembed()?;

    let mut points: Vec<PointStruct> = Vec::new();

    // Process files sequentially
    for file in file_metadata_list {
        // Check if file is already indexed
        if is_file_already_indexed(client, &file.path).await? {
            println!("File path '{}' already exists. Skipping.", file.path);
            continue;
        }

        // Generate summary sequentially to manage Ollama load
        let summary = match get_or_generate_summary(config, file, force_regenerate).await {
            Ok(sum) => sum,
            Err(e) => {
                eprintln!("Failed to generate summary for {}: {}", file.name, e);
                continue;
            }
        };

        // Generate embeddings
        let (dense_embeddings, _sparse_embeddings) =
            match generate_embeddings(summary.clone(), &model, &sparse_model).await {
                Ok(embeddings) => embeddings,
                Err(e) => {
                    eprintln!("Failed to generate embeddings for {}: {}", file.name, e);
                    continue;
                }
            };

        // Prepare payload
        let mut payload = Payload::new();
        payload.insert("file_name", Value::from(file.name.clone()));
        payload.insert("file_path", Value::from(file.path.clone()));
        payload.insert("file_size", Value::from(file.size as i64));
        payload.insert("summary", Value::from(summary));

        // Create point if embeddings are available
        if let Some(dense_embedding) = dense_embeddings.first() {
            let mut vectors_map: HashMap<String, Vec<f32>> = HashMap::new();
            vectors_map.insert("novum".to_string(), dense_embedding.clone());

            let uuid = Uuid::new_v4();
            let point = PointStruct::new(uuid.to_string(), vectors_map, payload);
            points.push(point);
        } else {
            eprintln!("No dense embeddings generated for file: {}", file.name);
        }
    }

    // Upsert points
    if !points.is_empty() {
        client
            .upsert_points(UpsertPoints {
                collection_name: "file_data".to_string(),
                wait: Some(true),
                points: points.clone(),
                ..Default::default()
            })
            .await
            .context("Failed to upsert points")?;

        println!("Points upserted successfully: {} files", points.len());
    } else {
        println!("No new files to upsert.");
    }

    Ok(())
}

/// Query the database using a vector and print matching file paths
pub async fn query_and_print_file_paths(
    client: &Qdrant,
    collection_name: &str,
    query_vector: Vec<f32>,
    limit: usize,
    use_sparse: bool, // Option to toggle between dense and sparse vector search
) -> anyhow::Result<()> {
    // Specify the vector type to use in the query
    let vector_type = if use_sparse { "splade" } else { "novum" };

    // Perform the query
    let query_result = client
        .query(
            QueryPointsBuilder::new(collection_name)
                .query(query_vector)
                .using(vector_type)
                .limit(limit as u64)
                .with_payload(true)
                .params(SearchParamsBuilder::default().hnsw_ef(128).exact(false)), // Configure search parameters
        )
        .await?;

    // Extract and print matching file paths
    for point in query_result.result {
        //if let Some(payload) = point.payload {
        //    if let Some(file_path) = payload.get("file_path").and_then(Value::as_str) {
        //        println!("File Path: {}", file_path);
        //    }
        //}
        println!("Result for Query: {:#?}", point);
    }

    Ok(())
}
