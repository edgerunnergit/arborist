# Arborist

Arborist is a command-line file management utility powered by machine learning. It allows you to efficiently scan, summarize, and search your files and folders using advanced natural language processing.

## Features

* **Directory Scanning:** Quickly scan directories, collecting metadata such as file counts, folder sizes, and extension distributions. Arborist intelligently handles hidden files and provides options to skip specified directories (e.g., `node_modules`, `.git`).
* **AI-Powered Summarization:**  Leverages the power of large language models (LLMs) to generate concise summaries of file and folder contents. Currently supports various document formats (`.pdf`, `.docx`, `.txt`, etc.), images, and more. Support for audio and video files is planned for the future.
* **Hybrid Search:**  Employs a hybrid search approach using both dense and sparse vector embeddings. This enables efficient semantic search of your files based on their content summaries. Arborist uses Qdrant as its vector database and fastembed for generating embeddings.
* **Extensible Design:** Designed with modularity and extensibility in mind. Future development will focus on adding support for more file types, integration with cloud storage, and enhanced search capabilities.

## Prerequisites

* **Rust and Cargo:**  Ensure you have Rust and Cargo installed. If not, follow the instructions at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
* **Ollama:** Arborist requires Ollama and the `gemma2:2b` model. Install Ollama by following the instructions on their official GitHub repository: [https://github.com/jmorganca/ollama](https://github.com/jmorganca/ollama). Then, download the `gemma2:2b` model using the Ollama CLI:

```bash
ollama pull gemma2:2b
```

## Installation

Once the prerequisites are met, install Arborist with:

```bash
cargo install --path .
```

## Usage

### Scanning and Summarizing

To scan and summarize a directory:

```bash
cargo run -- <path_to_directory> <query>
```

Replace `<path_to_directory>` with the path to the directory you want to analyze. The summary will be printed to the console.

### Searching

To search for files based on a query:

```bash
cargo run -- <path_to_directory> <query>  # The query will be matched against the generated summaries.
```

## Architecture

Arborist utilizes several key components:

* **`ollama-rs`:**  Interfaces with large language models (LLMs) like `gemma2:2b` for content summarization.
* **`qdrant-client`:**  Connects to the Qdrant vector database for storing and searching file summaries.
* **`fastembed`:** Generates dense and sparse vector embeddings of the summaries for efficient semantic search.
* **`pandoc`:** Used for converting various document formats to plain text for summarization.
* **`pdf-extract`:** Extracts text from PDF files.
* **`calamine`:**  Handles Microsoft Excel files (`.xlsx`).
* **`dotext`:** Processes PowerPoint (`.pptx`) and Word (`.docx`) files.

## Roadmap

* **Improved File Type Support:** Add summarization for audio, video, and archive files.
* **Cloud Storage Integration:** Allow scanning and indexing files stored in cloud services.
* **GUI Development:** Create a user-friendly graphical interface.
* **Performance Optimization:** Improve the efficiency of scanning and summarization.

## Contributing

Contributions are welcome! Feel free to open issues and submit pull requests.
