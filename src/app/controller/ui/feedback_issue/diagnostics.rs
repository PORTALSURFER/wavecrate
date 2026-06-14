use super::super::*;

impl AppController {
    pub(super) fn compose_issue_body(&self) -> Option<String> {
        let user_body = self.ui.feedback_issue.body.trim();
        let mut parts = Vec::new();
        if !user_body.is_empty() {
            parts.push(user_body.to_string());
        }
        parts.push(self.diagnostics_block());
        Some(parts.join("\n\n"))
    }

    fn diagnostics_block(&self) -> String {
        let version = env!("CARGO_PKG_VERSION");
        let build_type = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let logs = crate::app_dirs::logs_dir()
            .ok()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "n/a".to_string());
        format_feedback_diagnostics_block(version, build_type, os, arch, &logs)
    }
}

fn format_feedback_diagnostics_block(
    version: &str,
    build_type: &str,
    os: &str,
    arch: &str,
    logs_dir: &str,
) -> String {
    format!(
        "---\n\nDiagnostics\n- App version: {version}\n- OS: {os} ({arch})\n- Build: {build_type}\n- Logs dir: {logs_dir}\n- Latest run log: newest `*.log` file in Logs dir"
    )
}

#[cfg(test)]
mod tests {
    use super::format_feedback_diagnostics_block;

    #[test]
    fn diagnostics_block_points_to_log_directory_and_latest_log_hint() {
        let block = format_feedback_diagnostics_block(
            "1.2.3",
            "release",
            "windows",
            "x86_64",
            "C:\\Users\\me\\AppData\\Roaming\\.wavecrate\\logs",
        );

        assert!(block.contains("Logs dir: C:\\Users\\me\\AppData\\Roaming\\.wavecrate\\logs"));
        assert!(block.contains("Latest run log: newest `*.log` file in Logs dir"));
    }
}
