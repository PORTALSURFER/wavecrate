use serde::Serialize;
use std::path::{Path, PathBuf};

const DEFAULT_OUT_FILE: &str = "bench.json";
const DEFAULT_ANALYSIS_SAMPLES: usize = 200;
const DEFAULT_ANALYSIS_DURATION_MS: u32 = 500;
const DEFAULT_ANALYSIS_SAMPLE_RATE: u32 = 44_100;
const DEFAULT_SIMILARITY_ROWS: usize = 20_000;
const DEFAULT_GUI_ROWS: usize = 10_000;
/// Default row count used for focused GUI interaction scenarios.
const DEFAULT_GUI_INTERACTION_ROWS: usize = 2_000;
/// Default measured-iteration count per focused GUI interaction scenario.
const DEFAULT_GUI_INTERACTION_ITERS: usize = 24;
const DEFAULT_WARMUP_ITERS: usize = 5;
const DEFAULT_MEASURE_ITERS: usize = 30;

/// Runtime parameters for benchmark execution and output generation.
///
/// Values are validated before execution. The CLI defaults are explicit so
/// benchmark behavior is reproducible from run to run.
#[derive(Clone, Debug, Serialize)]
pub(super) struct BenchOptions {
    /// Destination path for the JSON report.
    pub(super) out: PathBuf,
    /// Enable analysis throughput benchmark.
    pub(super) analysis: bool,
    /// Include embedding inference in analysis benchmark.
    pub(super) analysis_full: bool,
    /// Enable similarity latency benchmark.
    pub(super) similarity: bool,
    /// Enable GUI frame/interaction benchmark.
    pub(super) gui: bool,
    /// RNG seed for synthetic fixtures.
    pub(super) seed: u64,
    /// Warmup iterations for each query.
    pub(super) warmup_iters: usize,
    /// Measured iterations for each query.
    pub(super) measure_iters: usize,
    /// Number of synthetic samples generated for analysis runs.
    pub(super) analysis_samples: usize,
    /// Synthetic audio duration in milliseconds.
    pub(super) analysis_duration_ms: u32,
    /// Synthetic audio sample rate in Hertz.
    pub(super) analysis_sample_rate: u32,
    /// Number of rows seeded for the similarity benchmark.
    pub(super) similarity_rows: usize,
    /// Number of rows seeded for the GUI benchmark.
    pub(super) gui_rows: usize,
    /// Number of rows used by focused interaction scenarios.
    pub(super) gui_interaction_rows: usize,
    /// Number of measured interaction iterations per scenario.
    pub(super) gui_interaction_iters: usize,
}

impl Default for BenchOptions {
    fn default() -> Self {
        Self {
            out: PathBuf::from(DEFAULT_OUT_FILE),
            analysis: true,
            analysis_full: false,
            similarity: true,
            gui: true,
            seed: 1,
            warmup_iters: DEFAULT_WARMUP_ITERS,
            measure_iters: DEFAULT_MEASURE_ITERS,
            analysis_samples: DEFAULT_ANALYSIS_SAMPLES,
            analysis_duration_ms: DEFAULT_ANALYSIS_DURATION_MS,
            analysis_sample_rate: DEFAULT_ANALYSIS_SAMPLE_RATE,
            similarity_rows: DEFAULT_SIMILARITY_ROWS,
            gui_rows: DEFAULT_GUI_ROWS,
            gui_interaction_rows: DEFAULT_GUI_INTERACTION_ROWS,
            gui_interaction_iters: DEFAULT_GUI_INTERACTION_ITERS,
        }
    }
}

/// Parse and validate benchmark CLI arguments.
///
/// Returns `None` when `-h` or `--help` is requested.
pub(super) fn parse_args(args: Vec<String>) -> Result<Option<BenchOptions>, String> {
    let mut options = BenchOptions::default();
    apply_args(&mut options, &args)?;
    if !options.analysis && !options.similarity && !options.gui {
        return Err("Nothing to benchmark: enable --analysis, --similarity, or --gui".to_string());
    }
    Ok(Some(options))
}

/// Persist a JSON report payload, creating directories as needed.
pub(super) fn write_output(path: &Path, payload: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Create output dir {} failed: {err}", parent.display()))?;
    }
    std::fs::write(path, payload)
        .map_err(|err| format!("Write output {} failed: {err}", path.display()))?;
    Ok(())
}

fn apply_args(options: &mut BenchOptions, args: &[String]) -> Result<(), String> {
    let mut idx = 0usize;
    while idx < args.len() {
        if apply_arg(options, args, &mut idx)? {
            return Ok(());
        }
        idx += 1;
    }
    Ok(())
}

fn apply_arg(options: &mut BenchOptions, args: &[String], idx: &mut usize) -> Result<bool, String> {
    let flag = args.get(*idx).map(String::as_str).unwrap_or_default();
    if flag == "-h" || flag == "--help" {
        println!("{}", help_text());
        return Ok(true);
    }
    if apply_toggle(options, flag) {
        return Ok(false);
    }
    if apply_value(options, args, idx, flag)? {
        return Ok(false);
    }
    Err(format!("Unknown argument: {flag}\n\n{}", help_text()))
}

fn apply_toggle(options: &mut BenchOptions, flag: &str) -> bool {
    match flag {
        "--analysis" => {
            options.analysis = true;
            true
        }
        "--no-analysis" => {
            options.analysis = false;
            true
        }
        "--analysis-full" => {
            options.analysis_full = true;
            true
        }
        "--analysis-lite" => {
            options.analysis_full = false;
            true
        }
        "--similarity" => {
            options.similarity = true;
            true
        }
        "--no-similarity" => {
            options.similarity = false;
            true
        }
        "--gui" => {
            options.gui = true;
            true
        }
        "--no-gui" => {
            options.gui = false;
            true
        }
        _ => false,
    }
}

fn apply_value(
    options: &mut BenchOptions,
    args: &[String],
    idx: &mut usize,
    flag: &str,
) -> Result<bool, String> {
    match flag {
        "--out" => {
            options.out = PathBuf::from(value_after(args, idx, "--out")?);
            Ok(true)
        }
        "--seed" => {
            options.seed = parse_u64(args, idx, "--seed")?;
            Ok(true)
        }
        "--warmup-iters" => {
            options.warmup_iters = parse_usize(args, idx, "--warmup-iters")?;
            Ok(true)
        }
        "--measure-iters" => {
            options.measure_iters = parse_usize(args, idx, "--measure-iters")?;
            Ok(true)
        }
        "--analysis-samples" => {
            options.analysis_samples = parse_usize(args, idx, "--analysis-samples")?;
            Ok(true)
        }
        "--analysis-duration-ms" => {
            options.analysis_duration_ms = parse_u32(args, idx, "--analysis-duration-ms")?;
            Ok(true)
        }
        "--analysis-sample-rate" => {
            options.analysis_sample_rate = parse_u32(args, idx, "--analysis-sample-rate")?;
            Ok(true)
        }
        "--similarity-rows" => {
            options.similarity_rows = parse_usize(args, idx, "--similarity-rows")?;
            Ok(true)
        }
        "--gui-rows" => {
            options.gui_rows = parse_usize(args, idx, "--gui-rows")?;
            Ok(true)
        }
        "--gui-interaction-rows" => {
            options.gui_interaction_rows = parse_usize(args, idx, "--gui-interaction-rows")?;
            Ok(true)
        }
        "--gui-interaction-iters" => {
            options.gui_interaction_iters = parse_usize(args, idx, "--gui-interaction-iters")?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_u64(args: &[String], idx: &mut usize, flag: &str) -> Result<u64, String> {
    let value = value_after(args, idx, flag)?;
    value
        .parse::<u64>()
        .map_err(|_| format!("Invalid {flag} value: {value}"))
}

fn parse_u32(args: &[String], idx: &mut usize, flag: &str) -> Result<u32, String> {
    let value = value_after(args, idx, flag)?;
    value
        .parse::<u32>()
        .map_err(|_| format!("Invalid {flag} value: {value}"))
}

fn parse_usize(args: &[String], idx: &mut usize, flag: &str) -> Result<usize, String> {
    let value = value_after(args, idx, flag)?;
    value
        .parse::<usize>()
        .map_err(|_| format!("Invalid {flag} value: {value}"))
}

fn value_after<'a>(args: &'a [String], idx: &mut usize, flag: &str) -> Result<&'a str, String> {
    *idx += 1;
    let value = args
        .get(*idx)
        .ok_or_else(|| format!("{flag} requires a value"))?;
    Ok(value)
}

fn help_text() -> &'static str {
    "Usage: sempal-bench [options]\n\n\
Options:\n\
  --out <path>                Output JSON path (default: bench.json)\n\
  --analysis / --no-analysis   Enable/disable analysis throughput bench (default: enabled)\n\
  --analysis-full              Include embedding inference in analysis bench\n\
  --analysis-lite              Skip embedding inference (feature-only)\n\
  --similarity / --no-similarity Enable/disable ANN similarity bench (default: enabled)\n\
  --gui / --no-gui               Enable/disable GUI frame/interaction bench (default: enabled)\n\
  --seed <u64>                 RNG seed for analysis fixtures (default: 1)\n\
  --warmup-iters <n>           Warmup iterations for each query (default: 5)\n\
  --measure-iters <n>          Measured iterations for each query (default: 30)\n\
  --analysis-samples <n>       Number of synthetic wavs to analyze (default: 200)\n\
  --analysis-duration-ms <ms>  Synthetic wav duration (default: 500)\n\
  --analysis-sample-rate <hz>  Synthetic wav sample rate (default: 44100)\n\
  --similarity-rows <n>        Seed rows for similarity benchmark (default: 20000)\n\
  --gui-rows <n>               Seed rows for GUI benchmark (default: 10000)\n\
  --gui-interaction-rows <n>   Seed rows for focused GUI interaction scenarios (default: 2000)\n\
  --gui-interaction-iters <n>  Measured iterations per interaction scenario (default: 24)\n\
  -h, --help                   Show this help\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse CLI args and require a concrete benchmark options payload.
    fn parse_options_or_panic(args: Vec<String>) -> BenchOptions {
        match parse_args(args) {
            Ok(Some(options)) => options,
            Ok(None) => panic!("expected options"),
            Err(err) => panic!("parse args failed: {err}"),
        }
    }

    /// Ensure default parse enables all benchmark groups and default values.
    #[test]
    fn parse_args_defaults_to_enabled_benchmarks() {
        let options = parse_options_or_panic(vec![]);
        assert!(options.analysis);
        assert!(options.similarity);
        assert!(options.gui);
        assert_eq!(options.out, PathBuf::from(DEFAULT_OUT_FILE));
        assert_eq!(options.analysis_samples, DEFAULT_ANALYSIS_SAMPLES);
        assert_eq!(options.analysis_duration_ms, DEFAULT_ANALYSIS_DURATION_MS);
        assert_eq!(options.analysis_sample_rate, DEFAULT_ANALYSIS_SAMPLE_RATE);
        assert_eq!(options.similarity_rows, DEFAULT_SIMILARITY_ROWS);
        assert_eq!(options.gui_rows, DEFAULT_GUI_ROWS);
        assert_eq!(options.gui_interaction_rows, DEFAULT_GUI_INTERACTION_ROWS);
        assert_eq!(options.gui_interaction_iters, DEFAULT_GUI_INTERACTION_ITERS);
    }

    /// Ensure toggle and output flags override benchmark defaults.
    #[test]
    fn parse_args_supports_toggle_and_output_flags() {
        let options = parse_options_or_panic(vec![
            "--no-analysis".to_string(),
            "--no-similarity".to_string(),
            "--analysis-full".to_string(),
            "--out".to_string(),
            "results.json".to_string(),
        ]);
        assert!(!options.analysis);
        assert!(!options.similarity);
        assert!(options.analysis_full);
        assert_eq!(options.out, PathBuf::from("results.json"));
    }

    /// Ensure parser rejects the fully disabled benchmark configuration.
    #[test]
    fn parse_args_rejects_all_disabled() {
        let err = parse_args(vec![
            "--no-analysis".to_string(),
            "--no-similarity".to_string(),
            "--no-gui".to_string(),
        ]);
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .contains("Nothing to benchmark: enable --analysis, --similarity, or --gui")
        );
    }

    /// Ensure parser reports numeric validation failures with flag context.
    #[test]
    fn parse_args_rejects_invalid_argument_value() {
        let err = parse_args(vec![
            "--analysis-samples".to_string(),
            "not-a-number".to_string(),
        ]);
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .contains("Invalid --analysis-samples value: not-a-number")
        );
    }

    /// Ensure parser reports unknown arguments with help text context.
    #[test]
    fn parse_args_rejects_unknown_argument() {
        let err = parse_args(vec!["--does-not-exist".to_string()]);
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .contains("Unknown argument: --does-not-exist")
        );
    }

    /// Ensure focused GUI interaction CLI overrides are parsed and applied.
    #[test]
    fn parse_args_accepts_gui_interaction_overrides() {
        let options = parse_options_or_panic(vec![
            "--gui-interaction-rows".to_string(),
            "128".to_string(),
            "--gui-interaction-iters".to_string(),
            "12".to_string(),
        ]);
        assert_eq!(options.gui_interaction_rows, 128);
        assert_eq!(options.gui_interaction_iters, 12);
    }
}
