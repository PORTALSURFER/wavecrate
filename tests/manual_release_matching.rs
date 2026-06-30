//! Public manual download-page release asset matching checks.

use regex::{Regex, RegexBuilder};

const MANUAL_INDEX: &str = include_str!("../manual/index.md");

#[test]
fn manual_release_asset_pattern_accepts_current_supported_platforms() {
    let pattern = manual_release_asset_pattern();

    for name in [
        "wavecrate-v1.2.3-windows-x86_64.zip",
        "wavecrate-v1.2.3-macos-x86_64.zip",
        "wavecrate-v1.2.3-macos-aarch64.zip",
    ] {
        assert!(pattern.is_match(name), "{name} should match");
    }
}

#[test]
fn manual_release_asset_pattern_rejects_unsupported_linux_assets() {
    let pattern = manual_release_asset_pattern();

    for name in [
        "wavecrate-v1.2.3-linux-x86_64.zip",
        "wavecrate-v1.2.3-linux-aarch64.zip",
    ] {
        assert!(
            !pattern.is_match(name),
            "{name} must not be treated as a current public download"
        );
    }
}

#[test]
fn manual_download_copy_names_only_current_download_platforms() {
    let copy = "Download the latest Windows or macOS portable bundle.";

    assert!(MANUAL_INDEX.contains(copy));
    assert!(
        !MANUAL_INDEX.contains("Linux portable bundle"),
        "current download copy must not imply Linux support"
    );
}

fn manual_release_asset_pattern() -> Regex {
    let js_pattern = MANUAL_INDEX
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('/')
                .and_then(|rest| rest.strip_suffix("/i;"))
        })
        .expect("manual release asset regex literal");

    RegexBuilder::new(js_pattern)
        .case_insensitive(true)
        .build()
        .expect("manual release asset regex compiles as Rust regex")
}
