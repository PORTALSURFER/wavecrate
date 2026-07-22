use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    app_dirs::{AppRootGuard, PersistenceProfileGuard},
    sample_sources::{
        SampleSource, SourceId,
        config::{AppConfig, AppSettingsCore},
    },
};

use super::{
    FixtureManifest, FixtureName, FixtureProfile, FixtureSourceManifest,
    audio::{AudioSpec, write_deterministic_wav},
    manifest_io::{file_manifest, sha256, write_json},
    specification::{FIXTURE_SEED, FIXTURE_VERSION},
    topology::definitions,
};

const MANIFEST_FILE_NAME: &str = "fixture-manifest.json";
const READINESS_TARGETS_PER_FILE: usize = 3;

/// Input for deterministic native source-fixture provisioning.
#[derive(Clone, Debug)]
pub struct FixtureProvisionRequest {
    /// Config base that owns the isolated profile and disposable source roots.
    pub config_base: PathBuf,
    /// Stable fixture to provision.
    pub fixture: FixtureName,
    /// Non-live persistence profile to populate.
    pub profile: FixtureProfile,
    /// Reconstruct the profile and source roots from a clean baseline.
    pub reset: bool,
}

/// Provision or validate one deterministic fixture and isolated profile.
pub fn provision(request: &FixtureProvisionRequest) -> Result<FixtureManifest, String> {
    let config_base = prepare_config_base(&request.config_base)?;
    let profile_path = profile_path(&config_base, request.profile);
    let fixture_root = fixture_root(&config_base, request.fixture);
    ensure_confined(&profile_path, &config_base, "profile")?;
    ensure_confined(&fixture_root, &config_base, "fixture")?;
    ensure_reset_path_safe(&profile_path, &config_base, "profile")?;
    ensure_reset_path_safe(&fixture_root, &config_base, "fixture")?;

    if !request.reset && fixture_root.join(MANIFEST_FILE_NAME).is_file() {
        return validate(&config_base, request.fixture, request.profile);
    }
    if request.reset {
        remove_tree_if_present(&profile_path)?;
        remove_tree_if_present(&fixture_root)?;
    }
    fs::create_dir_all(&fixture_root)
        .map_err(|error| format!("create fixture root {}: {error}", fixture_root.display()))?;

    let definitions = definitions(request.fixture);
    let mut sources = Vec::with_capacity(definitions.len());
    let mut source_manifests = Vec::with_capacity(definitions.len());
    let mut files = Vec::new();
    for definition in definitions {
        let source_root = fixture_root.join(definition.directory_name);
        fs::create_dir_all(&source_root)
            .map_err(|error| format!("create source root {}: {error}", source_root.display()))?;
        for relative in &definition.directories {
            fs::create_dir_all(source_root.join(relative)).map_err(|error| {
                format!(
                    "create fixture directory {relative} in {}: {error}",
                    source_root.display()
                )
            })?;
        }
        for generated in &definition.audio {
            let path = source_root.join(&generated.relative_path);
            write_deterministic_wav(
                &path,
                &AudioSpec {
                    channels: generated.channels,
                    sample_rate: generated.sample_rate,
                    frames: generated.frames,
                    seed: generated.seed,
                },
            )?;
            files.push(file_manifest(
                definition.id,
                &generated.relative_path,
                &path,
                true,
                Some(generated),
            )?);
        }
        for (relative_path, contents) in &definition.unsupported {
            let path = source_root.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "create unsupported fixture parent {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&path, contents)
                .map_err(|error| format!("write fixture file {}: {error}", path.display()))?;
            files.push(file_manifest(
                definition.id,
                relative_path,
                &path,
                false,
                None,
            )?);
        }

        let supported_file_count = definition.audio.len();
        let unsupported_file_count = definition.unsupported.len();
        let expected_readiness_target_count = supported_file_count
            .saturating_mul(READINESS_TARGETS_PER_FILE)
            .saturating_add(usize::from(supported_file_count > 0));
        source_manifests.push(FixtureSourceManifest {
            source_id: definition.id.to_owned(),
            directory_name: definition.directory_name.to_owned(),
            root: source_root.clone(),
            directories: definition.directories,
            supported_file_count,
            unsupported_file_count,
            expected_readiness_target_count,
        });
        sources.push(SampleSource::new_with_id(
            SourceId::from_string(definition.id),
            source_root,
        ));
    }

    files.sort_by(|left, right| {
        (&left.source_id, &left.relative_path).cmp(&(&right.source_id, &right.relative_path))
    });
    let _profile_guard = PersistenceProfileGuard::named(request.profile.as_str());
    let _root_guard = AppRootGuard::set(profile_path.clone())
        .map_err(|error| format!("select fixture profile {}: {error}", profile_path.display()))?;
    let mut core = AppSettingsCore::default();
    core.last_selected_source = sources.first().map(|source| source.id.clone());
    crate::sample_sources::config::save(&AppConfig {
        sources: sources.clone(),
        core,
    })
    .map_err(|error| format!("save fixture profile {}: {error}", profile_path.display()))?;
    for source in &sources {
        let database = source.open_db().map_err(|error| {
            format!(
                "initialize fixture source DB {}: {error}",
                source.root.display()
            )
        })?;
        crate::sample_sources::scanner::scan_once(&database).map_err(|error| {
            format!(
                "seed fixture source manifest {}: {error}",
                source.root.display()
            )
        })?;
    }

    let manifest = FixtureManifest {
        fixture_version: FIXTURE_VERSION,
        fixture: request.fixture,
        deterministic_seed: FIXTURE_SEED,
        profile: request.profile,
        profile_path,
        fixture_root: fixture_root.clone(),
        expected_supported_file_count: source_manifests
            .iter()
            .map(|source| source.supported_file_count)
            .sum(),
        expected_unsupported_file_count: source_manifests
            .iter()
            .map(|source| source.unsupported_file_count)
            .sum(),
        expected_readiness_target_count: source_manifests
            .iter()
            .map(|source| source.expected_readiness_target_count)
            .sum(),
        sources: source_manifests,
        files,
    };
    write_json(&fixture_root.join(MANIFEST_FILE_NAME), &manifest)?;
    validate(&config_base, request.fixture, request.profile)
}

/// Validate one fixture manifest, generated inputs, directories, and configured sources.
pub fn validate(
    config_base: &Path,
    fixture: FixtureName,
    profile: FixtureProfile,
) -> Result<FixtureManifest, String> {
    let config_base = prepare_config_base(config_base)?;
    let expected_fixture_root = fixture_root(&config_base, fixture);
    let manifest_path = expected_fixture_root.join(MANIFEST_FILE_NAME);
    let bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read fixture manifest {}: {error}", manifest_path.display()))?;
    let manifest: FixtureManifest = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "parse fixture manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    if manifest.fixture_version != FIXTURE_VERSION
        || manifest.fixture != fixture
        || manifest.profile != profile
        || manifest.deterministic_seed != FIXTURE_SEED
        || manifest.fixture_root != expected_fixture_root
        || manifest.profile_path != profile_path(&config_base, profile)
    {
        return Err(format!(
            "fixture manifest {} does not match the requested version, name, seed, profile, or roots",
            manifest_path.display()
        ));
    }
    for source in &manifest.sources {
        ensure_confined(&source.root, &manifest.fixture_root, "source")?;
        if !source.root.join(".wavecrate.db").is_file() {
            return Err(format!(
                "fixture source database is missing: {}",
                source.root.join(".wavecrate.db").display()
            ));
        }
        for directory in &source.directories {
            let path = source.root.join(directory);
            if !path.is_dir() {
                return Err(format!("fixture directory is missing: {}", path.display()));
            }
        }
        let expected_files = manifest
            .files
            .iter()
            .filter(|file| file.source_id == source.source_id)
            .map(|file| file.relative_path.clone())
            .collect::<BTreeSet<_>>();
        let actual_files = fixture_input_files(&source.root)?;
        if actual_files != expected_files {
            return Err(format!(
                "fixture file inventory mismatch for {}: expected {:?}, got {:?}",
                source.root.display(),
                expected_files,
                actual_files
            ));
        }
    }
    for file in &manifest.files {
        let source = manifest
            .sources
            .iter()
            .find(|source| source.source_id == file.source_id)
            .ok_or_else(|| format!("manifest file references unknown source {}", file.source_id))?;
        let path = source.root.join(&file.relative_path);
        let actual = sha256(&path)?;
        if actual != file.sha256 {
            return Err(format!(
                "fixture hash mismatch for {}: expected {}, got {}",
                path.display(),
                file.sha256,
                actual
            ));
        }
    }
    let supported_count = manifest
        .files
        .iter()
        .filter(|file| file.supported_audio)
        .count();
    let unsupported_count = manifest.files.len().saturating_sub(supported_count);
    let readiness_count = manifest
        .sources
        .iter()
        .map(|source| source.expected_readiness_target_count)
        .sum::<usize>();
    if supported_count != manifest.expected_supported_file_count
        || unsupported_count != manifest.expected_unsupported_file_count
        || readiness_count != manifest.expected_readiness_target_count
    {
        return Err(String::from(
            "fixture manifest aggregate counts do not match its file and source entries",
        ));
    }

    let _profile_guard = PersistenceProfileGuard::named(profile.as_str());
    let expected_profile_path = profile_path(&config_base, profile);
    let _root_guard = AppRootGuard::set(expected_profile_path.clone()).map_err(|error| {
        format!(
            "select fixture profile {}: {error}",
            expected_profile_path.display()
        )
    })?;
    let configured = crate::sample_sources::config::load_or_default()
        .map_err(|error| format!("load provisioned fixture profile: {error}"))?;
    let configured_pairs = configured
        .sources
        .iter()
        .map(|source| (source.id.as_str(), source.root.as_path()))
        .collect::<Vec<_>>();
    let manifest_pairs = manifest
        .sources
        .iter()
        .map(|source| (source.source_id.as_str(), source.root.as_path()))
        .collect::<Vec<_>>();
    if configured_pairs != manifest_pairs {
        return Err(String::from(
            "configured fixture sources do not match the manifest source ids and roots",
        ));
    }
    Ok(manifest)
}

#[cfg(test)]
pub(super) fn manifest_path(config_base: &Path, fixture: FixtureName) -> PathBuf {
    fixture_root(config_base, fixture).join(MANIFEST_FILE_NAME)
}

fn prepare_config_base(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() || path.parent().is_none() {
        return Err(format!(
            "refusing unsafe fixture config base {}",
            path.display()
        ));
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("create fixture config base {}: {error}", path.display()))?;
    path.canonicalize()
        .map_err(|error| format!("resolve fixture config base {}: {error}", path.display()))
}

fn profile_path(config_base: &Path, profile: FixtureProfile) -> PathBuf {
    config_base
        .join(crate::app_dirs::APP_DIR_NAME)
        .join(crate::app_dirs::PROFILE_DIR_NAME)
        .join(profile.as_str())
}

fn fixture_root(config_base: &Path, fixture: FixtureName) -> PathBuf {
    config_base
        .join(crate::app_dirs::APP_DIR_NAME)
        .join("fixtures")
        .join(fixture.as_str())
}

fn ensure_confined(path: &Path, parent: &Path, label: &str) -> Result<(), String> {
    if !path.starts_with(parent) || path == parent {
        return Err(format!(
            "refusing {label} path outside its configured fixture boundary: {}",
            path.display()
        ));
    }
    Ok(())
}

fn ensure_reset_path_safe(path: &Path, config_base: &Path, label: &str) -> Result<(), String> {
    let relative = path.strip_prefix(config_base).map_err(|error| {
        format!(
            "resolve {label} path {} below fixture config base: {error}",
            path.display()
        )
    })?;
    let mut current = config_base.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "refusing {label} path through symbolic link: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(format!(
                    "inspect {label} path component {}: {error}",
                    current.display()
                ));
            }
        }
    }
    Ok(())
}

fn remove_tree_if_present(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("reset fixture path {}: {error}", path.display()))?;
    }
    Ok(())
}

fn fixture_input_files(root: &Path) -> Result<BTreeSet<String>, String> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeSet<String>) -> Result<(), String> {
        let entries = fs::read_dir(directory)
            .map_err(|error| format!("read fixture directory {}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!("read fixture entry in {}: {error}", directory.display())
            })?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("read fixture entry type: {error}"))?;
            if file_type.is_dir() {
                visit(root, &entry.path(), files)?;
            } else if file_type.is_file() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if matches!(
                    name.as_ref(),
                    ".wavecrate.db" | ".wavecrate.db-wal" | ".wavecrate.db-shm"
                ) {
                    continue;
                }
                let entry_path = entry.path();
                let relative = entry_path.strip_prefix(root).map_err(|error| {
                    format!(
                        "resolve fixture relative path {}: {error}",
                        entry_path.display()
                    )
                })?;
                files.insert(relative.to_string_lossy().replace('\\', "/"));
            } else {
                return Err(format!(
                    "fixture contains unsupported filesystem entry: {}",
                    entry.path().display()
                ));
            }
        }
        Ok(())
    }

    let mut files = BTreeSet::new();
    visit(root, root, &mut files)?;
    Ok(files)
}
