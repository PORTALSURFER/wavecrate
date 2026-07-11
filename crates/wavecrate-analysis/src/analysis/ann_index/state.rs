use crate::analysis::{similarity, version};
use hnsw_rs::hnswio::HnswIo;
use hnsw_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    mem::ManuallyDrop,
    path::{Path, PathBuf},
    time::Instant,
};

#[cfg(test)]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
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
    pub(crate) last_flush: Instant,
    pub(crate) dirty_inserts: usize,
}

/// HNSW storage that either owns all point data directly or keeps its disk
/// loader alive for as long as a loaded graph can reference mmap-backed data.
pub(crate) enum AnnHnsw {
    Built(Hnsw<'static, f32, DistCosine>),
    Loaded(LoadedAnnHnsw),
}

impl AnnHnsw {
    fn graph(&self) -> &Hnsw<'static, f32, DistCosine> {
        match self {
            Self::Built(hnsw) => hnsw,
            Self::Loaded(hnsw) => hnsw.graph(),
        }
    }

    pub(crate) fn insert(&self, embedding_with_id: (&[f32], usize)) {
        self.graph().insert(embedding_with_id);
    }

    pub(crate) fn search(&self, embedding: &[f32], requested: usize, ef: usize) -> Vec<Neighbour> {
        self.graph().search(embedding, requested, ef)
    }

    pub(crate) fn file_dump(&self, dir: &Path, basename: &str) -> Result<String, String> {
        self.graph()
            .file_dump(dir, basename)
            .map_err(|err| err.to_string())
    }

    pub(crate) fn get_nb_point(&self) -> usize {
        self.graph().get_nb_point()
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
    #[cfg(test)]
    live_probe: Option<Arc<AtomicUsize>>,
}

impl LoadedAnnHnsw {
    pub(crate) fn load(dir: &Path, basename: &str) -> Result<Self, String> {
        Self::load_inner(dir, basename, None)
    }

    #[cfg(test)]
    pub(crate) fn load_with_live_probe(
        dir: &Path,
        basename: &str,
        live_probe: Arc<AtomicUsize>,
    ) -> Result<Self, String> {
        Self::load_inner(dir, basename, Some(live_probe))
    }

    fn load_inner(
        dir: &Path,
        basename: &str,
        #[cfg(test)] live_probe: Option<Arc<AtomicUsize>>,
        #[cfg(not(test))] _live_probe: Option<()>,
    ) -> Result<Self, String> {
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
        #[cfg(test)]
        if let Some(probe) = &live_probe {
            probe.fetch_add(1, Ordering::AcqRel);
        }
        Ok(Self {
            hnsw: ManuallyDrop::new(hnsw),
            _loader: loader,
            #[cfg(test)]
            live_probe,
        })
    }

    fn graph(&self) -> &Hnsw<'static, f32, DistCosine> {
        &self.hnsw
    }

    #[cfg(test)]
    pub(crate) fn get_nb_point(&self) -> usize {
        self.graph().get_nb_point()
    }
}

impl Drop for LoadedAnnHnsw {
    fn drop(&mut self) {
        // SAFETY: `hnsw` is initialized exactly once and this is its only
        // explicit drop. `_loader` remains alive until after this method.
        unsafe {
            ManuallyDrop::drop(&mut self.hnsw);
        }
        #[cfg(test)]
        if let Some(probe) = &self.live_probe {
            probe.fetch_sub(1, Ordering::AcqRel);
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
