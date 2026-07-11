use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;

use wavecrate::sample_sources::{
    STARMAP_LAYOUT_UMAP_VERSION, StarmapLayoutLoadRequest, StarmapLayoutSample,
    StarmapSourceLayoutRequest,
};

use super::{FileEntry, FolderBrowserState};

impl FolderBrowserState {
    pub(super) fn starmap_layout_load_request(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> (StarmapLayoutLoadRequest, usize) {
        let snapshot = self.browser_listing_snapshot(tags_by_file);
        let listed_count = snapshot.rows().len();
        let mut by_source: HashMap<String, StarmapSourceLayoutRequest> = HashMap::new();
        for file in snapshot.rows() {
            let path = Path::new(&file.id);
            let Some((source, relative_path)) = self.sample_source_for_file_path(path) else {
                continue;
            };
            let sample_id = build_sample_id(source.id.as_str(), &relative_path);
            by_source
                .entry(source.id.as_str().to_string())
                .or_insert_with(|| StarmapSourceLayoutRequest {
                    source,
                    samples: Vec::new(),
                })
                .samples
                .push(StarmapLayoutSample {
                    file_id: file.id.clone(),
                    sample_id,
                });
        }
        let signature = starmap_layout_signature(self.selected_source_id(), snapshot.rows());
        (
            StarmapLayoutLoadRequest {
                signature,
                sources: by_source.into_values().collect(),
            },
            listed_count,
        )
    }
}

fn build_sample_id(source_id: &str, relative_path: &Path) -> String {
    format!(
        "{}::{}",
        source_id,
        relative_path.to_string_lossy().replace('\\', "/")
    )
}

pub(super) fn starmap_layout_signature(source_id: &str, files: &[&FileEntry]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_id.hash(&mut hasher);
    STARMAP_LAYOUT_UMAP_VERSION.hash(&mut hasher);
    files.len().hash(&mut hasher);
    for file in files {
        file.id.hash(&mut hasher);
    }
    hasher.finish()
}
