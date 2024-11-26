use crate::file_management::FileMetadata;
use crate::utils::setup_fastembed;
use anyhow::Result;
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

pub async fn process_and_upload_files(
    client: &Qdrant,
    file_metadata_list: &Vec<FileMetadata>,
) -> Result<()> {
    let (model, sparse_model) = setup_fastembed()?;

    let mut points: Vec<PointStruct> = Vec::new();

    for file in file_metadata_list {
        // Check if the file path already exists in the collection
        let query_result = client
            .query(
                QueryPointsBuilder::new("file_data")
                    .filter(Filter::must([Condition::matches(
                        "file_path",
                        file.path.clone(),
                    )]))
                    .limit(1),
            )
            .await;

        if let Ok(response) = query_result {
            if !response.result.is_empty() {
                println!("File path '{}' already exists. Skipping.", file.path);
                continue;
            }
        } else {
            eprintln!(
                "Error querying file path '{}': {:?}. Skipping.",
                file.path, query_result
            );
            continue;
        }

        let summary = file.summary.clone();

        // Generate both dense and sparse embeddings
        let (dense_embeddings, sparse_embeddings) =
            match generate_embeddings(summary, &model, &sparse_model).await {
                Ok(embeddings) => embeddings,
                Err(err) => {
                    eprintln!("Error generating embeddings for {}: {}", file.name, err);
                    continue; // Skip to the next file if embeddings fail
                }
            };

        // Prepare payload
        let mut payload = Payload::new();
        payload.insert("file_name", Value::from(file.name.clone()));
        payload.insert("file_path", Value::from(file.path.clone()));
        payload.insert("file_size", Value::from(file.size as i64));

        if let (Some(dense_embedding), Some(_sparse_embedding)) =
            (dense_embeddings.first(), sparse_embeddings.first())
        {
            let mut vectors_map: HashMap<String, Vec<f32>> = HashMap::new();
            vectors_map.insert("novum".to_string(), dense_embedding.clone());

            // TODO: fix adding sparse vectors
            //let sparse_values: Vec<f32> = sparse_embedding.values.to_vec(); // Extract values
            //vectors_map.insert("splade".to_string(), sparse_values);

            let uuid = Uuid::new_v4();

            let point = PointStruct::new(uuid.to_string(), vectors_map, payload.clone());
            points.push(point);
        } else {
            eprintln!(
                "Error: Could not generate embeddings for file: {}",
                file.name
            );
        }
    }
    // Now upsert the collected points
    match client
        .upsert_points(UpsertPoints {
            collection_name: "file_data".to_string(),
            wait: Some(true),
            points,
            ..Default::default() // Use default values for other fields
        })
        .await
    {
        Ok(_) => println!("Points upserted successfully!"),
        Err(err) => eprintln!("Error upserting points: {}", err),
    };

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
