//! Build script for platform-specific build configuration.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/windows/wavecrate.rc");
    println!("cargo:rerun-if-changed=build/windows/wavecrate.exe.manifest");
    println!("cargo:rerun-if-changed=assets/logo3.ico");
    println!("cargo:rerun-if-env-changed=WAVECRATE_GIT_SHA");
    println!("cargo:rerun-if-env-changed=WAVECRATE_BUILD_NUMBER");

    emit_git_rerun_hints();
    emit_git_sha();
    emit_build_number();

    if compiling_for_windows_target()
        && let Err(error) = compile_windows_resources()
    {
        eprintln!("Failed to embed Windows resources: {error}");
        std::process::exit(1);
    }
}

fn compiling_for_windows_target() -> bool {
    env::var("CARGO_CFG_TARGET_OS")
        .map(|target| target == "windows")
        .unwrap_or_else(|_| cfg!(target_os = "windows"))
}

fn compile_windows_resources() -> Result<(), Box<dyn std::error::Error>> {
    embed_resource::compile("build/windows/wavecrate.rc", embed_resource::NONE)
        .manifest_optional()?;
    Ok(())
}

fn emit_git_rerun_hints() {
    let Some(head_path) = resolve_git_path("HEAD") else {
        return;
    };
    println!("cargo:rerun-if-changed={}", head_path.display());

    if let Some(reference_path) = resolve_head_reference_path(&head_path) {
        println!("cargo:rerun-if-changed={}", reference_path.display());
    }
}

fn emit_git_sha() {
    let git_sha = env::var("WAVECRATE_GIT_SHA")
        .ok()
        .and_then(trim_nonempty)
        .or_else(resolve_git_sha)
        .unwrap_or_else(|| String::from("<unknown>"));
    println!("cargo:rustc-env=WAVECRATE_BUILD_GIT_SHA={git_sha}");
}

fn emit_build_number() {
    let build_number = env::var("WAVECRATE_BUILD_NUMBER")
        .ok()
        .and_then(valid_build_number)
        .or_else(resolve_git_commit_count)
        .unwrap_or_else(|| String::from("0"));
    println!("cargo:rustc-env=WAVECRATE_BUILD_NUMBER={build_number}");
}

fn resolve_head_reference_path(head_path: &Path) -> Option<PathBuf> {
    let head_contents = fs::read_to_string(head_path).ok()?;
    let reference = head_contents.trim().strip_prefix("ref: ")?;
    resolve_git_path(reference)
}

fn resolve_git_path(reference: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-path", reference])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let git_path = String::from_utf8(output.stdout).ok()?;
    let trimmed = git_path.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn resolve_git_sha() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let git_sha = String::from_utf8(output.stdout).ok()?;
    trim_nonempty(git_sha)
}

fn resolve_git_commit_count() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let count = String::from_utf8(output.stdout).ok()?;
    valid_build_number(count)
}

fn valid_build_number(value: impl AsRef<str>) -> Option<String> {
    let trimmed = value.as_ref().trim();
    if !trimmed.is_empty() && trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn trim_nonempty(value: impl AsRef<str>) -> Option<String> {
    let trimmed = value.as_ref().trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
