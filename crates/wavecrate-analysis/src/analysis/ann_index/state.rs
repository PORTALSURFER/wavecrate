use crate::analysis::{similarity, version};
use hnsw_rs::hnswio::HnswIo;
use hnsw_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    mem::ManuallyDrop,
    path::{Path, PathBuf},
};

/// Configuration parameters for ANN index building/loading.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct AnnIndexParams {
    pub(crate) analysis_version: String,
    pub(crate) model_id: String,
    pub(crate) metric: String,
    pub(crate) dim: usize,
    pub(crate) max_nb_connection: usize,
    pub(crate) ef_construction: usize,
    pub(crate) ef_search: usize,
    pub(crate) max_layer: usize,
}

/// In-memory ANN index state with metadata for persistence.
pub(crate) struct AnnIndexState {
    pub(crate) hnsw: AnnHnsw,
    pub(crate) id_map: Vec<String>,
    pub(crate) id_lookup: HashMap<String, usize>,
    pub(crate) params: AnnIndexParams,
    pub(crate) index_path: PathBuf,
}

/// HNSW storage that either owns all point data directly or keeps its disk
/// loader alive for as long as a loaded graph can reference mmap-backed data.
pub(crate) enum AnnHnsw {
    Built(Hnsw<'static, f32, DistCosine>),
    Loaded(LoadedAnnHnsw),
}

impl AnnHnsw {
    pub(crate) fn search(&self, embedding: &[f32], requested: usize, ef: usize) -> Vec<Neighbour> {
        match self {
            Self::Built(hnsw) => hnsw.search(embedding, requested, ef),
            Self::Loaded(hnsw) => hnsw.search(embedding, requested, ef),
        }
    }

    pub(crate) fn file_dump(&self, dir: &Path, basename: &str) -> Result<String, String> {
        match self {
            Self::Built(hnsw) => hnsw.file_dump(dir, basename).map_err(|err| err.to_string()),
            Self::Loaded(hnsw) => hnsw.file_dump(dir, basename),
        }
    }

    pub(crate) fn get_nb_point(&self) -> usize {
        match self {
            Self::Built(hnsw) => hnsw.get_nb_point(),
            Self::Loaded(hnsw) => hnsw.get_nb_point(),
        }
    }
}

/// Self-referential disk-loaded HNSW owner.
///
/// `HnswIo::load_hnsw` ties the graph lifetime to the loader because points may
/// borrow its mmap. The loader lives at a stable boxed address, and `Drop`
/// destroys the graph before Rust drops that loader.
pub(crate) struct LoadedAnnHnsw {
    hnsw: ManuallyDrop<Hnsw<'static, f32, DistCosine>>,
    _loader: Box<HnswIo>,
}

impl LoadedAnnHnsw {
    pub(crate) fn load(dir: &Path, basename: &str) -> Result<Self, String> {
        let mut loader = Box::new(HnswIo::new(dir, basename));
        let hnsw = loader
            .load_hnsw::<f32, DistCosine>()
            .map_err(|_| "Failed to read ANN index".to_string())?;
        // SAFETY: `Hnsw` may borrow only from the boxed `HnswIo`. Moving this
        // wrapper never moves the loader allocation, callers can use the graph
        // only through scoped operations that return no borrowed point data,
        // and our Drop implementation destroys the graph first.
        let hnsw = unsafe {
            std::mem::transmute::<Hnsw<'_, f32, DistCosine>, Hnsw<'static, f32, DistCosine>>(hnsw)
        };
        Ok(Self {
            hnsw: ManuallyDrop::new(hnsw),
            _loader: loader,
        })
    }

    fn search(&self, embedding: &[f32], requested: usize, ef: usize) -> Vec<Neighbour> {
        self.hnsw.search(embedding, requested, ef)
    }

    fn file_dump(&self, dir: &Path, basename: &str) -> Result<String, String> {
        self.hnsw
            .file_dump(dir, basename)
            .map_err(|err| err.to_string())
    }

    pub(crate) fn get_nb_point(&self) -> usize {
        self.hnsw.get_nb_point()
    }
}

impl Drop for LoadedAnnHnsw {
    fn drop(&mut self) {
        // SAFETY: `hnsw` is initialized exactly once and this is its only
        // explicit drop. `_loader` remains alive until after this method.
        unsafe {
            ManuallyDrop::drop(&mut self.hnsw);
        }
    }
}

/// Metadata persisted in the source database for ANN indexes.
pub(crate) struct AnnIndexMetaRow {
    pub(crate) index_path: PathBuf,
    pub(crate) params: AnnIndexParams,
}

/// Default ANN parameters for similarity embeddings.
pub(crate) fn default_params() -> AnnIndexParams {
    AnnIndexParams {
        analysis_version: version::analysis_version().to_string(),
        model_id: similarity::SIMILARITY_MODEL_ID.to_string(),
        metric: "cosine".to_string(),
        dim: similarity::SIMILARITY_DIM,
        max_nb_connection: 16,
        ef_construction: 200,
        ef_search: 64,
        max_layer: 16,
    }
}

/// Build a lookup map from sample ids to ANN point ids.
pub(crate) fn build_id_lookup(id_map: &[String]) -> HashMap<String, usize> {
    let mut lookup = HashMap::with_capacity(id_map.len());
    for (idx, sample_id) in id_map.iter().enumerate() {
        lookup.insert(sample_id.clone(), idx);
    }
    lookup
}
