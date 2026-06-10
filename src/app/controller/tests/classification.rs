//! Classification inventory for mature controller test clusters.
//!
//! These tests remain valuable while native/app-core coverage is still being
//! filled in. The inventory makes the intended owner and next action explicit
//! without re-expanding the retired UI path.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControllerTestClassification {
    ActiveProductContract,
    MigrationEvidence,
    CompatibilityCoverage,
    ImplementationDetailReview,
}

#[derive(Clone, Copy, Debug)]
struct ControllerTestCluster {
    path: &'static str,
    classification: ControllerTestClassification,
    owner: &'static str,
    next_action: &'static str,
}

const CONTROLLER_TEST_CLUSTERS: &[ControllerTestCluster] = &[
    ControllerTestCluster {
        path: "src/app/controller/library/wavs/browser_pipeline/tests.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "app-core browser projection and browser-search pipeline tests",
        next_action: "Keep as blocking coverage until browser filtering, sorting, similarity, and duplicate cleanup are mirrored in app-core/native tests.",
    },
    ControllerTestCluster {
        path: "src/app/controller/library/browser_controller/helpers/sample_mutation_tests.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "sample metadata and file-mutation service tests",
        next_action: "Port behavior slices when mutation services are separated from AppController; do not delete before equivalent service tests exist.",
    },
    ControllerTestCluster {
        path: "src/app/controller/library/wavs/browser_search_worker/pipeline/stages_tests.rs",
        classification: ControllerTestClassification::ImplementationDetailReview,
        owner: "browser-search worker pipeline tests",
        next_action: "Retain while the worker owns cache invalidation; collapse into behavior tests once the staged pipeline has a smaller public contract.",
    },
    ControllerTestCluster {
        path: "src/app/controller/playback/waveform_action_tests.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "native/app-core waveform action and bridge tests",
        next_action: "Mirror selection, cursor, zoom, loop, and edit behavior in app-core/native tests before retiring controller-level cases.",
    },
    ControllerTestCluster {
        path: "src/app/controller/library/browser_controller/actions/metadata/tests.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "metadata editing and tag library tests",
        next_action: "Keep as blocking metadata workflow coverage; port to app-core metadata DTO/service tests as ownership moves.",
    },
    ControllerTestCluster {
        path: "src/app/controller/library/wavs/metadata_async/tests.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "background metadata job tests",
        next_action: "Retain until async metadata work is isolated behind an app-core service with deterministic job tests.",
    },
    ControllerTestCluster {
        path: "src/app/controller/library/background_jobs/scan/tests.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "source scanning and background-job tests",
        next_action: "Keep as product-critical source scan coverage; port only after scan orchestration leaves AppController.",
    },
    ControllerTestCluster {
        path: "src/app/controller/library/source_folders/delete_recovery/recovery/tests.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "folder delete recovery tests",
        next_action: "Keep as blocking recovery coverage until native/app-core file-operation recovery tests cover restore and purge behavior.",
    },
    ControllerTestCluster {
        path: "src/app/controller/tests/browser_core/tagging.rs",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "tagging workflow tests",
        next_action: "Retain until tag assignment, normal tags, and rating interactions are covered by app-core/native metadata workflow tests.",
    },
    ControllerTestCluster {
        path: "src/app/controller/tests/source_config.rs",
        classification: ControllerTestClassification::CompatibilityCoverage,
        owner: "config migration and source startup compatibility tests",
        next_action: "Keep explicit compatibility coverage because legacy pane collapse protects user config migration.",
    },
    ControllerTestCluster {
        path: "src/app/controller/tests/drag_drop_*",
        classification: ControllerTestClassification::MigrationEvidence,
        owner: "native drag/drop and app-core projection tests",
        next_action: "Use as migration evidence while native drag/drop coverage matures; retire only when native tests cover browser, source, folder, and waveform drops.",
    },
    ControllerTestCluster {
        path: "src/app/controller/library/selection_export/selection_export_tests/*",
        classification: ControllerTestClassification::ActiveProductContract,
        owner: "selection export and sample extraction tests",
        next_action: "Keep as extraction-loop coverage until export rendering and naming are covered outside AppController.",
    },
];

#[test]
fn controller_test_cluster_inventory_classifies_major_clusters() {
    assert!(CONTROLLER_TEST_CLUSTERS.len() >= 10);
    for cluster in CONTROLLER_TEST_CLUSTERS {
        assert!(!cluster.path.is_empty());
        assert!(
            !cluster.owner.is_empty(),
            "missing owner for {}",
            cluster.path
        );
        assert!(
            !cluster.next_action.is_empty(),
            "missing next action for {}",
            cluster.path
        );
    }
}

#[test]
fn controller_test_cluster_inventory_keeps_product_coverage_blocking() {
    let active_contracts = CONTROLLER_TEST_CLUSTERS
        .iter()
        .filter(|cluster| {
            cluster.classification == ControllerTestClassification::ActiveProductContract
        })
        .count();
    assert!(
        active_contracts >= 7,
        "most large controller test clusters still protect product behavior"
    );
}

#[test]
fn controller_test_cluster_inventory_names_non_product_contracts_explicitly() {
    assert!(CONTROLLER_TEST_CLUSTERS.iter().any(|cluster| {
        cluster.classification == ControllerTestClassification::MigrationEvidence
    }));
    assert!(CONTROLLER_TEST_CLUSTERS.iter().any(|cluster| {
        cluster.classification == ControllerTestClassification::CompatibilityCoverage
    }));
    assert!(CONTROLLER_TEST_CLUSTERS.iter().any(|cluster| {
        cluster.classification == ControllerTestClassification::ImplementationDetailReview
    }));
}
