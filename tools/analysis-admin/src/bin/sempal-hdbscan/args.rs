//! CLI parsing and option validation for the HDBSCAN helper binary.

use std::path::PathBuf;

/// Parsed runtime options for the HDBSCAN helper.
#[derive(Debug, Clone)]
pub(super) struct Options {
    pub(super) db_path: Option<PathBuf>,
    pub(super) model_id: String,
    pub(super) min_cluster_size: usize,
    pub(super) min_samples: Option<usize>,
    pub(super) allow_single_cluster: bool,
    pub(super) min_noise_ratio: f32,
    pub(super) max_noise_ratio: f32,
    pub(super) noise_policy: NoisePolicy,
}

/// Policy for handling out-of-bounds noise ratios after clustering.
#[derive(Debug, Clone, Copy)]
pub(super) enum NoisePolicy {
    Warn,
    Error,
}

impl NoisePolicy {
    pub(super) fn handle_ratio(&self, ratio: f32, min: f32, max: f32) -> Result<(), String> {
        if ratio < min || ratio > max {
            let message = format!(
                "Noise ratio {:.2}% outside [{:.2}%, {:.2}%]",
                ratio * 100.0,
                min * 100.0,
                max * 100.0
            );
            return match self {
                Self::Warn => {
                    eprintln!("Warning: {message}");
                    Ok(())
                }
                Self::Error => Err(message),
            };
        }
        Ok(())
    }
}

/// Parse binary arguments into validated runtime options.
pub(super) fn parse_args(args: Vec<String>) -> Result<Option<Options>, String> {
    let mut options = Options {
        db_path: None,
        model_id: String::new(),
        min_cluster_size: 5,
        min_samples: None,
        allow_single_cluster: false,
        min_noise_ratio: 0.0,
        max_noise_ratio: 0.95,
        noise_policy: NoisePolicy::Warn,
    };

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "-h" | "--help" => {
                println!("{}", help_text());
                return Ok(None);
            }
            "--db" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--db requires a value".to_string())?;
                options.db_path = Some(PathBuf::from(value));
            }
            "--model-id" => {
                idx += 1;
                options.model_id = args
                    .get(idx)
                    .ok_or_else(|| "--model-id requires a value".to_string())?
                    .to_string();
            }
            "--min-cluster-size" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--min-cluster-size requires a value".to_string())?;
                options.min_cluster_size = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid --min-cluster-size value: {value}"))?;
            }
            "--min-samples" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--min-samples requires a value".to_string())?;
                options.min_samples = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid --min-samples value: {value}"))?,
                );
            }
            "--allow-single-cluster" => {
                options.allow_single_cluster = true;
            }
            "--min-noise" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--min-noise requires a value".to_string())?;
                options.min_noise_ratio = value
                    .parse::<f32>()
                    .map_err(|_| format!("Invalid --min-noise value: {value}"))?;
            }
            "--max-noise" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--max-noise requires a value".to_string())?;
                options.max_noise_ratio = value
                    .parse::<f32>()
                    .map_err(|_| format!("Invalid --max-noise value: {value}"))?;
            }
            "--noise-policy" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--noise-policy requires a value".to_string())?;
                options.noise_policy = match value.as_str() {
                    "warn" => NoisePolicy::Warn,
                    "error" => NoisePolicy::Error,
                    _ => {
                        return Err(format!(
                            "Invalid --noise-policy {value}. Use warn or error."
                        ));
                    }
                };
            }
            unknown => return Err(format!("Unknown argument: {unknown}\n\n{}", help_text())),
        }
        idx += 1;
    }

    validate_options(&options)?;
    Ok(Some(options))
}

fn validate_options(options: &Options) -> Result<(), String> {
    if options.model_id.trim().is_empty() {
        return Err("--model-id is required".to_string());
    }
    if !(0.0..=1.0).contains(&options.min_noise_ratio) {
        return Err("--min-noise must be between 0.0 and 1.0".to_string());
    }
    if !(0.0..=1.0).contains(&options.max_noise_ratio) {
        return Err("--max-noise must be between 0.0 and 1.0".to_string());
    }
    if options.min_noise_ratio > options.max_noise_ratio {
        return Err("--min-noise must be <= --max-noise".to_string());
    }
    Ok(())
}

fn help_text() -> String {
    [
        "sempal-hdbscan",
        "",
        "Build HDBSCAN clusters for embeddings.",
        "",
        "Usage:",
        "  sempal-hdbscan --model-id <id> [options]",
        "",
        "Options:",
        "  --db <path>              Path to library.db (defaults to app data location).",
        "  --model-id <id>          Embedding model id to read (required).",
        "  --min-cluster-size <n>   Minimum cluster size (default: 5).",
        "  --min-samples <n>        Min samples for core distance (default: min_cluster_size).",
        "  --allow-single-cluster   Allow a single giant cluster.",
        "  --min-noise <f>          Warn/error if noise ratio below (default: 0.0).",
        "  --max-noise <f>          Warn/error if noise ratio above (default: 0.95).",
        "  --noise-policy <mode>    warn or error (default: warn).",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{NoisePolicy, parse_args};

    #[test]
    fn parse_args_rejects_unknown_noise_policy() {
        let err = parse_args(vec![
            "--model-id".to_string(),
            "model".to_string(),
            "--noise-policy".to_string(),
            "loud".to_string(),
        ])
        .expect_err("unknown policy should fail");
        assert_eq!(err, "Invalid --noise-policy loud. Use warn or error.");
    }

    #[test]
    fn parse_args_rejects_noise_range_inversion() {
        let err = parse_args(vec![
            "--model-id".to_string(),
            "model".to_string(),
            "--min-noise".to_string(),
            "0.7".to_string(),
            "--max-noise".to_string(),
            "0.4".to_string(),
        ])
        .expect_err("inverted noise bounds should fail");
        assert_eq!(err, "--min-noise must be <= --max-noise");
    }

    #[test]
    fn noise_policy_warn_allows_out_of_bounds_ratio() {
        assert!(NoisePolicy::Warn.handle_ratio(1.0, 0.0, 0.5).is_ok());
    }
}
