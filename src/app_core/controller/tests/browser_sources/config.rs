use super::*;

#[test]
fn apply_configuration_prunes_transient_benchmark_sources() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let retained_root = match tempdir() {
        Ok(dir) => {
            let root = dir.path().join("user-source");
            if let Err(err) = std::fs::create_dir_all(&root) {
                panic!("failed to create retained fixture: {err}");
            }
            std::mem::forget(dir);
            root
        }
        Err(err) => panic!("failed to create retained tempdir: {err}"),
    };
    let transient_root = std::env::temp_dir()
        .join("sempal-test-gui-source")
        .join("gui-source");
    if let Err(err) = std::fs::create_dir_all(&transient_root) {
        panic!("failed to create transient fixture: {err}");
    }
    let cfg = crate::sample_sources::config::AppConfig {
        sources: vec![
            crate::sample_sources::SampleSource::new(transient_root),
            crate::sample_sources::SampleSource::new(retained_root.clone()),
        ],
        ..crate::sample_sources::config::AppConfig::default()
    };

    if let Err(err) = controller.apply_configuration(cfg) {
        panic!("failed to apply configuration: {err}");
    }

    assert_eq!(controller.ui.sources.rows.len(), 1);
    assert_eq!(
        controller.ui.sources.rows[0].path,
        retained_root.to_string_lossy()
    );
}
