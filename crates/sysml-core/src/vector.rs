use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use hnsw_rs::prelude::*;

use crate::element::SysmlElement;

const MAX_NB_CONNECTION: usize = 16;
const MAX_LAYER: usize = 16;
const EF_CONSTRUCTION: usize = 200;
const SEARCH_EF: usize = 64;

pub struct VectorIndex {
    model: Arc<Mutex<TextEmbedding>>,
    embeddings: Arc<Vec<Vec<f32>>>,
    element_ids: Arc<Vec<String>>,
    hnsw: Arc<Hnsw<'static, f32, DistCosine>>,
}

impl Clone for VectorIndex {
    fn clone(&self) -> Self {
        Self {
            model: Arc::clone(&self.model),
            embeddings: Arc::clone(&self.embeddings),
            element_ids: Arc::clone(&self.element_ids),
            hnsw: Arc::clone(&self.hnsw),
        }
    }
}

fn load_model() -> Result<TextEmbedding, String> {
    TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
    )
    .map_err(|e| format!("Failed to load embedding model: {e}"))
}

fn embed_query(model: &Mutex<TextEmbedding>, query: &str) -> Result<Vec<f32>, String> {
    let mut m = model
        .lock()
        .map_err(|e| format!("Model lock poisoned: {e}"))?;
    let result = m
        .embed(vec![query], None)
        .map_err(|e| format!("Query embedding failed: {e}"))?;
    result
        .into_iter()
        .next()
        .ok_or_else(|| "Empty embedding result".to_string())
}

fn format_element_text(kind: &str, qualified_name: &str, doc: Option<&str>) -> String {
    match doc {
        Some(d) => format!("{kind}: {qualified_name}. {d}"),
        None => format!("{kind}: {qualified_name}"),
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        let x = *x as f64;
        let y = *y as f64;
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

impl VectorIndex {
    pub fn build(elements: &[SysmlElement]) -> Result<Self, String> {
        let mut model = load_model()?;

        let mut texts = Vec::new();
        let mut element_ids = Vec::new();

        for elem in elements {
            let text = format_element_text(&elem.kind, &elem.qualified_name, elem.doc.as_deref());
            texts.push(text);
            element_ids.push(elem.qualified_name.clone());
        }

        let embeddings = if texts.is_empty() {
            Vec::new()
        } else {
            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            model
                .embed(text_refs, None)
                .map_err(|e| format!("Embedding failed: {e}"))?
        };

        let hnsw: Hnsw<'static, f32, DistCosine> = Hnsw::new(
            MAX_NB_CONNECTION,
            embeddings.len().max(1),
            MAX_LAYER,
            EF_CONSTRUCTION,
            DistCosine,
        );
        for (i, emb) in embeddings.iter().enumerate() {
            hnsw.insert((emb.as_slice(), i));
        }

        Ok(VectorIndex {
            model: Arc::new(Mutex::new(model)),
            embeddings: Arc::new(embeddings),
            element_ids: Arc::new(element_ids),
            hnsw: Arc::new(hnsw),
        })
    }

    pub fn search(&self, query: &str, top_k: usize) -> Result<Vec<SemanticMatch>, String> {
        if self.embeddings.is_empty() {
            return Ok(Vec::new());
        }

        let query_vec = embed_query(&self.model, query)?;
        let neighbours = self.hnsw.search(&query_vec, top_k, SEARCH_EF);

        let results = neighbours
            .into_iter()
            .map(|n| {
                let qn = &self.element_ids[n.d_id];
                SemanticMatch {
                    qualified_name: qn.clone(),
                    similarity: 1.0 - n.distance as f64,
                }
            })
            .collect();

        Ok(results)
    }

    pub fn element_similarities(&self, query: &str) -> Result<HashMap<String, f64>, String> {
        if self.embeddings.is_empty() {
            return Ok(HashMap::new());
        }

        let query_vec = embed_query(&self.model, query)?;

        let mut sims = HashMap::new();
        for (i, emb) in self.embeddings.iter().enumerate() {
            let sim = cosine_similarity(&query_vec, emb).max(0.0);
            if sim > 0.0 {
                sims.insert(self.element_ids[i].clone(), sim);
            }
        }

        Ok(sims)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SemanticMatch {
    pub qualified_name: String,
    pub similarity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    static EVE_VECTOR_INDEX: LazyLock<VectorIndex> = LazyLock::new(|| {
        use nomograph_core::traits::KnowledgeGraph;
        let results = crate::graph::tests::parse_all_eve();
        let mut graph = crate::SysmlGraph::new();
        graph.index(results).unwrap();
        VectorIndex::build(graph.elements()).unwrap()
    });

    #[test]
    fn test_format_element_text() {
        assert_eq!(
            format_element_text("part_definition", "Vehicle::Engine", None),
            "part_definition: Vehicle::Engine"
        );
        assert_eq!(
            format_element_text("part_definition", "Vehicle::Engine", Some("Main engine")),
            "part_definition: Vehicle::Engine. Main engine"
        );
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_vector_index_build() {
        let vi = &*EVE_VECTOR_INDEX;
        assert!(vi.embeddings.len() > 0);
        assert_eq!(vi.embeddings.len(), vi.element_ids.len());
        for emb in vi.embeddings.iter() {
            assert_eq!(emb.len(), 384);
        }
    }

    #[test]
    fn test_semantic_search() {
        let vi = &*EVE_VECTOR_INDEX;
        let results = vi.search("shield protection requirement", 5).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].similarity > 0.0);
        assert!(results[0].similarity <= 1.0 + 1e-6);
    }

    #[test]
    fn test_element_similarities() {
        let vi = &*EVE_VECTOR_INDEX;
        let sims = vi
            .element_similarities("mining frigate requirements")
            .unwrap();
        assert!(!sims.is_empty());
        for score in sims.values() {
            assert!(*score >= 0.0 && *score <= 1.0 + 1e-6);
        }
    }

    #[test]
    fn test_empty_index() {
        let vi = VectorIndex::build(&[]).unwrap();
        assert!(vi.embeddings.is_empty());
        let results = vi.search("anything", 5).unwrap();
        assert!(results.is_empty());
        let sims = vi.element_similarities("anything").unwrap();
        assert!(sims.is_empty());
    }
}
