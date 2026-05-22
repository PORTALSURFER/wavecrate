use super::*;

impl BrowserController<'_> {
    pub(in crate::app::controller::library::browser_controller::actions::metadata) fn resolve_auto_rename_target(
        &self,
        root: &Path,
        relative_path: &Path,
        tagged_basename: Option<&str>,
        fallback_identifier: &str,
        reserved_targets: &mut HashSet<PathBuf>,
    ) -> Result<PathBuf, String> {
        resolve_auto_rename_target(
            root,
            relative_path,
            tagged_basename,
            fallback_identifier,
            reserved_targets,
            |relative_path, basename| self.name_with_preserved_extension(relative_path, basename),
            |relative_path, root, full_name| {
                self.validate_new_sample_name_in_parent(relative_path, root, full_name)
            },
        )
    }
}

pub(super) fn resolve_auto_rename_target_for_worker(
    root: &Path,
    relative_path: &Path,
    tagged_basename: Option<&str>,
    fallback_identifier: &str,
    reserved_targets: &mut HashSet<PathBuf>,
) -> Result<PathBuf, String> {
    resolve_auto_rename_target(
        root,
        relative_path,
        tagged_basename,
        fallback_identifier,
        reserved_targets,
        name_with_preserved_extension_for_worker,
        validate_new_sample_name_in_parent_for_worker,
    )
}

fn resolve_auto_rename_target(
    root: &Path,
    relative_path: &Path,
    tagged_basename: Option<&str>,
    fallback_identifier: &str,
    reserved_targets: &mut HashSet<PathBuf>,
    name_with_extension: impl Fn(&Path, &str) -> Result<String, String>,
    validate_name: impl Fn(&Path, &Path, &str) -> Result<PathBuf, String>,
) -> Result<PathBuf, String> {
    if let Some(tagged_basename) = tagged_basename {
        if let Some(path) = try_auto_rename_target(
            root,
            relative_path,
            tagged_basename,
            reserved_targets,
            &name_with_extension,
            &validate_name,
        )? {
            reserved_targets.insert(path.clone());
            return Ok(path);
        }
        for index in 1..=999 {
            let suffixed_basename = format!("{tagged_basename}_{index:03}");
            if let Some(path) = try_auto_rename_target(
                root,
                relative_path,
                &suffixed_basename,
                reserved_targets,
                &name_with_extension,
                &validate_name,
            )? {
                reserved_targets.insert(path.clone());
                return Ok(path);
            }
        }
    }
    for index in 1..=999 {
        let fallback_basename = format!("{fallback_identifier}_{index:03}");
        if let Some(path) = try_auto_rename_target(
            root,
            relative_path,
            &fallback_basename,
            reserved_targets,
            &name_with_extension,
            &validate_name,
        )? {
            reserved_targets.insert(path.clone());
            return Ok(path);
        }
    }
    Err(format!(
        "Unable to find a unique auto-rename target for {}",
        relative_path.display()
    ))
}

fn try_auto_rename_target(
    root: &Path,
    relative_path: &Path,
    basename: &str,
    reserved_targets: &HashSet<PathBuf>,
    name_with_extension: impl Fn(&Path, &str) -> Result<String, String>,
    validate_name: impl Fn(&Path, &Path, &str) -> Result<PathBuf, String>,
) -> Result<Option<PathBuf>, String> {
    let full_name = name_with_extension(relative_path, basename)?;
    let new_relative = validate_name(relative_path, root, &full_name);
    match new_relative {
        Ok(path) if path == relative_path || !reserved_targets.contains(&path) => Ok(Some(path)),
        Ok(_) => Ok(None),
        Err(err) if err.contains("already exists") => Ok(None),
        Err(err) => Err(err),
    }
}

fn name_with_preserved_extension_for_worker(
    current_relative: &Path,
    new_name: &str,
) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    let Some(ext) = current_relative.extension().and_then(|ext| ext.to_str()) else {
        return Ok(trimmed.to_string());
    };
    let ext_lower = ext.to_ascii_lowercase();
    let should_strip_suffix = |suffix: &str| -> bool {
        let suffix_lower = suffix.to_ascii_lowercase();
        suffix_lower == ext_lower
            || matches!(
                suffix_lower.as_str(),
                "wav" | "wave" | "flac" | "aif" | "aiff" | "mp3" | "ogg" | "opus"
            )
    };
    let stem = if let Some((stem, suffix)) = trimmed.rsplit_once('.') {
        if !stem.is_empty() && should_strip_suffix(suffix) {
            stem
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let stem = stem.trim_end_matches('.');
    if stem.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    Ok(format!("{stem}.{ext}"))
}

fn validate_new_sample_name_in_parent_for_worker(
    relative_path: &Path,
    root: &Path,
    new_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.contains(['/', '\\']) {
        return Err("Name cannot contain path separators".into());
    }
    let parent = relative_path.parent().unwrap_or(Path::new(""));
    let new_relative = parent.join(trimmed);
    let new_absolute = root.join(&new_relative);
    if new_absolute.exists() && new_relative != relative_path {
        return Err(format!(
            "A file named {} already exists",
            new_relative.display()
        ));
    }
    Ok(new_relative)
}
