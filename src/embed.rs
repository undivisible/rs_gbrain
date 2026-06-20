//! Chunk embeddings for hybrid RAG.

use anyhow::Result;

pub const EMBED_DIM: usize = 384;

pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dim(&self) -> usize {
        EMBED_DIM
    }
}

/// Deterministic hash embedder for tests and CI (no model download).
pub struct HashEmbedder;

impl Embedder for HashEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| hash_to_vec(t)).collect())
    }
}

fn hash_to_vec(text: &str) -> Vec<f32> {
    let mut v = vec![0f32; EMBED_DIM];
    for (i, b) in text.as_bytes().iter().enumerate() {
        v[i % EMBED_DIM] += (*b as f32) / 255.0;
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

#[cfg(feature = "fastembed")]
pub struct FastembedModel {
    model: fastembed::TextEmbedding,
}

#[cfg(feature = "fastembed")]
impl FastembedModel {
    pub fn new() -> Result<Self> {
        use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;
        Ok(Self { model })
    }
}

#[cfg(feature = "fastembed")]
impl Embedder for FastembedModel {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let owned: Vec<String> = texts.iter().map(|s| (*s).to_string()).collect();
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        Ok(self.model.embed(refs, None)?)
    }
}

pub fn f32_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

pub fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for i in 0..n {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let d = na.sqrt() * nb.sqrt();
    if d == 0.0 {
        0.0
    } else {
        dot / d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_embed_stable() {
        let e = HashEmbedder;
        let a = e.embed(&["hello"]).unwrap();
        let b = e.embed(&["hello"]).unwrap();
        assert_eq!(a[0].len(), EMBED_DIM);
        assert!((cosine(&a[0], &b[0]) - 1.0).abs() < 0.001);
    }
}
