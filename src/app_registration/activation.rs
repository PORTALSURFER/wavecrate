use super::entitlement::SignedEntitlement;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct LocalActivation {
    pub(super) install_id: String,
    pub(super) device_id: String,
    pub(super) last_entitlement: Option<SignedEntitlement>,
}

pub(super) fn load_or_create_activation() -> Result<LocalActivation, String> {
    let path = activation_file()?;
    if path.exists() {
        let bytes = fs::read(&path).map_err(|err| {
            format!(
                "failed to read Wavecrate registration file at {}: {err}",
                path.display()
            )
        })?;
        return serde_json::from_slice(&bytes).map_err(|err| {
            format!(
                "failed to parse Wavecrate registration file at {}: {err}",
                path.display()
            )
        });
    }

    Ok(LocalActivation {
        install_id: Uuid::new_v4().to_string(),
        device_id: Uuid::new_v4().to_string(),
        last_entitlement: None,
    })
}

pub(super) fn save_activation(activation: &LocalActivation) -> Result<(), String> {
    let path = activation_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create Wavecrate registration directory at {}: {err}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(activation)
        .map_err(|err| format!("failed to encode Wavecrate registration file: {err}"))?;
    fs::write(&path, bytes).map_err(|err| {
        format!(
            "failed to write Wavecrate registration file at {}: {err}",
            path.display()
        )
    })
}

fn activation_file() -> Result<PathBuf, String> {
    let dir = wavecrate::app_dirs::app_root_dir()
        .map_err(|err| format!("failed to resolve Wavecrate app data directory: {err}"))?
        .join("registration");
    Ok(dir.join("activation.json"))
}
