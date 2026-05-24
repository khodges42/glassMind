use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Embedding {
    pub model: String,
    pub vector: Vec<f32>,
}

pub trait EmbeddingBackend {
    fn model(&self) -> &str;
    fn embed(&self, text: &str) -> Result<Embedding>;
}

pub struct LocalHashEmbedding {
    model: String,
    dimensions: usize,
}

impl LocalHashEmbedding {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            dimensions: 64,
        }
    }
}

impl EmbeddingBackend for LocalHashEmbedding {
    fn model(&self) -> &str {
        &self.model
    }

    fn embed(&self, text: &str) -> Result<Embedding> {
        Ok(Embedding {
            model: self.model.clone(),
            vector: hash_embedding(text, self.dimensions),
        })
    }
}

pub struct OllamaEmbedding {
    model: String,
    url: String,
}

impl OllamaEmbedding {
    pub fn new(model: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            url: url.into(),
        }
    }
}

impl EmbeddingBackend for OllamaEmbedding {
    fn model(&self) -> &str {
        &self.model
    }

    fn embed(&self, text: &str) -> Result<Embedding> {
        // For now this keeps the pipeline local and testable. The backend shape is here, and
        // the HTTP call can replace this body without touching retrieval or storage code.
        let seed = format!("{}:{}:{}", self.url, self.model, text);
        Ok(Embedding {
            model: self.model.clone(),
            vector: hash_embedding(&seed, 64),
        })
    }
}

pub fn backend_from_config(config: &crate::config::Config) -> Box<dyn EmbeddingBackend> {
    match config.embeddings.backend.as_str() {
        "ollama" => Box::new(OllamaEmbedding::new(
            config.embeddings.model.clone(),
            config.embeddings.url.clone(),
        )),
        _ => Box::new(LocalHashEmbedding::new(config.embeddings.model.clone())),
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut a_norm = 0.0;
    let mut b_norm = 0.0;

    for (left, right) in a.iter().zip(b.iter()) {
        dot += left * right;
        a_norm += left * left;
        b_norm += right * right;
    }

    if a_norm == 0.0 || b_norm == 0.0 {
        return 0.0;
    }

    dot / (a_norm.sqrt() * b_norm.sqrt())
}

fn hash_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut vector = vec![0.0; dimensions];

    for token in text.split_whitespace() {
        let normalized = token
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if normalized.is_empty() {
            continue;
        }

        let hash = Sha256::digest(normalized.as_bytes());
        let idx = usize::from(hash[0]) % dimensions;
        let sign = if hash[1] % 2 == 0 { 1.0 } else { -1.0 };
        vector[idx] += sign;
    }

    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }

    vector
}
