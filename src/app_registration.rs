use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, io::Read, path::PathBuf, time::Duration};
use uuid::Uuid;

const ACTIVATION_BASE_URL: &str = "https://portalsurfer.org/wavecrate/api/v1";
const APP_ID: &str = "wavecrate";
const MAX_LICENSE_RESPONSE_BYTES: usize = 32 * 1024;
const RETRY_COUNT: usize = 2;

const BUILD_ID: Option<&str> = option_env!("WAVECRATE_BUILD_ID");
const BUILD_SIGNATURE: Option<&str> = option_env!("WAVECRATE_BUILD_SIGNATURE");
const SIGNING_PUBLIC_KEY_B64: Option<&str> = option_env!("WAVECRATE_SIGNING_PUBLIC_KEY_B64");

#[derive(Debug, Deserialize, Serialize)]
struct LocalActivation {
    install_id: String,
    device_id: String,
    last_entitlement: Option<SignedEntitlement>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SignedEntitlement {
    entitlement: Value,
    signature: String,
    signature_algorithm: String,
}

#[derive(Debug, Deserialize)]
struct LicenseResponse {
    entitlement: Value,
    signature: String,
    signature_algorithm: String,
}

#[derive(Debug, Serialize)]
struct LicenseRequest<'a> {
    app: &'a str,
    install_id: &'a str,
    device_id: &'a str,
    app_version: &'a str,
    build_id: &'a str,
    build_signature: &'a str,
}

#[derive(Clone, Copy)]
struct RegisteredBuild {
    build_id: &'static str,
    build_signature: &'static str,
    public_key_b64: &'static str,
}

pub(crate) fn ensure_registration() -> Result<(), String> {
    let build = registered_build()?;
    let mut activation = load_or_create_activation()?;
    match renew_or_activate(build, &activation, false) {
        Ok(entitlement) => {
            activation.last_entitlement = Some(entitlement);
            save_activation(&activation)?;
            Ok(())
        }
        Err(first_error) => {
            if !confirm_activation_request(build.build_id, &first_error) {
                return Err(String::from("Wavecrate access was not activated"));
            }
            match renew_or_activate(build, &activation, true) {
                Ok(entitlement) => {
                    show_info(
                        "Wavecrate access",
                        &activation_success_message(&entitlement),
                    );
                    activation.last_entitlement = Some(entitlement);
                    save_activation(&activation)?;
                    Ok(())
                }
                Err(err) => {
                    show_error("Wavecrate access", &err);
                    Err(err)
                }
            }
        }
    }
}

fn registered_build() -> Result<RegisteredBuild, String> {
    match (BUILD_ID, BUILD_SIGNATURE, SIGNING_PUBLIC_KEY_B64) {
        (Some(build_id), Some(build_signature), Some(public_key_b64)) => Ok(RegisteredBuild {
            build_id,
            build_signature,
            public_key_b64,
        }),
        _ => Err(String::from(
            "incomplete Wavecrate activation build metadata; set WAVECRATE_BUILD_ID, \
             WAVECRATE_BUILD_SIGNATURE, and WAVECRATE_SIGNING_PUBLIC_KEY_B64",
        )),
    }
}

fn load_or_create_activation() -> Result<LocalActivation, String> {
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

fn save_activation(activation: &LocalActivation) -> Result<(), String> {
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

fn renew_or_activate(
    build: RegisteredBuild,
    activation: &LocalActivation,
    force_activate: bool,
) -> Result<SignedEntitlement, String> {
    let request = LicenseRequest {
        app: APP_ID,
        install_id: &activation.install_id,
        device_id: &activation.device_id,
        app_version: env!("CARGO_PKG_VERSION"),
        build_id: build.build_id,
        build_signature: build.build_signature,
    };
    let endpoint = if force_activate || activation.last_entitlement.is_none() {
        "activate"
    } else {
        "lease"
    };
    let response = post_license_request(endpoint, &request).or_else(|err| {
        if endpoint == "lease" {
            post_license_request("activate", &request)
        } else {
            Err(err)
        }
    })?;
    verify_entitlement(build, response)
}

fn post_license_request(
    endpoint: &str,
    request: &LicenseRequest<'_>,
) -> Result<LicenseResponse, String> {
    let url = format!("{ACTIVATION_BASE_URL}/{endpoint}");
    let mut last_error = None;
    for attempt in 0..RETRY_COUNT {
        match post_license_request_once(&url, request) {
            Ok(response) => return Ok(response),
            Err(err) => {
                last_error = Some(err);
                if attempt + 1 < RETRY_COUNT {
                    std::thread::sleep(Duration::from_millis(350));
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| String::from("license request failed")))
}

fn post_license_request_once(
    url: &str,
    request: &LicenseRequest<'_>,
) -> Result<LicenseResponse, String> {
    let response = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
        .post(url)
        .set("Content-Type", "application/json")
        .send_json(serde_json::to_value(request).map_err(|err| err.to_string())?);

    match response {
        Ok(response) => read_license_response(response),
        Err(ureq::Error::Status(code, response)) => {
            let body =
                read_response_text(response).unwrap_or_else(|_| String::from("<unreadable>"));
            Err(format!("server returned HTTP {code}: {body}"))
        }
        Err(ureq::Error::Transport(err)) => Err(format!("network error: {err}")),
    }
}

fn read_license_response(response: ureq::Response) -> Result<LicenseResponse, String> {
    let text = read_response_text(response)?;
    serde_json::from_str(&text).map_err(|err| format!("invalid license response: {err}"))
}

fn read_response_text(response: ureq::Response) -> Result<String, String> {
    let mut reader = response
        .into_reader()
        .take(u64::try_from(MAX_LICENSE_RESPONSE_BYTES).unwrap_or(u64::MAX) + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read license response: {err}"))?;
    if bytes.len() > MAX_LICENSE_RESPONSE_BYTES {
        return Err(String::from("license response was too large"));
    }
    String::from_utf8(bytes).map_err(|err| format!("license response was not UTF-8: {err}"))
}

fn verify_entitlement(
    build: RegisteredBuild,
    response: LicenseResponse,
) -> Result<SignedEntitlement, String> {
    if response.signature_algorithm != "Ed25519" {
        return Err(format!(
            "unsupported license signature algorithm: {}",
            response.signature_algorithm
        ));
    }
    let public_key = decode_public_key(build.public_key_b64)?;
    let signature_bytes = general_purpose::STANDARD
        .decode(&response.signature)
        .map_err(|err| format!("invalid license signature encoding: {err}"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|err| format!("invalid license signature: {err}"))?;
    let payload = stable_json(&response.entitlement);
    public_key
        .verify(payload.as_bytes(), &signature)
        .map_err(|err| format!("license signature did not verify: {err}"))?;

    let app = response
        .entitlement
        .get("app")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let status = response
        .entitlement
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let response_build = response
        .entitlement
        .get("build_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if app != APP_ID || response_build != build.build_id || status != "active" {
        return Err(format!(
            "Wavecrate entitlement is not active for this build: app={app}, build={response_build}, status={status}"
        ));
    }

    Ok(SignedEntitlement {
        entitlement: response.entitlement,
        signature: response.signature,
        signature_algorithm: response.signature_algorithm,
    })
}

fn decode_public_key(value: &str) -> Result<VerifyingKey, String> {
    let bytes = general_purpose::STANDARD
        .decode(value)
        .map_err(|err| format!("invalid Wavecrate public key encoding: {err}"))?;
    let key_bytes = bytes
        .get(bytes.len().saturating_sub(32)..)
        .ok_or_else(|| String::from("Wavecrate public key is too short"))?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| String::from("Wavecrate public key has invalid length"))?;
    VerifyingKey::from_bytes(&key_array)
        .map_err(|err| format!("invalid Wavecrate public key: {err}"))
}

fn stable_json(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.to_string(),
        Value::Array(values) => {
            let inner = values.iter().map(stable_json).collect::<Vec<_>>().join(",");
            format!("[{inner}]")
        }
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            let inner = entries
                .into_iter()
                .map(|(key, value)| {
                    format!("{}:{}", Value::String(key.clone()), stable_json(value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

fn activation_success_message(entitlement: &SignedEntitlement) -> String {
    let lease = entitlement
        .entitlement
        .get("lease_expires_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let access = entitlement
        .entitlement
        .get("access_expires_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    format!(
        "Wavecrate access granted. Lease expires {lease}; access expires {access}. Launching Wavecrate."
    )
}

fn confirm_activation_request(build_id: &str, reason: &str) -> bool {
    let description = format!(
        "This Wavecrate build needs server access before it can launch.\n\nBuild: {build_id}\n\nReason: {reason}\n\nRequest access for this computer now?"
    );
    MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Wavecrate access")
        .set_description(&description)
        .set_buttons(MessageButtons::OkCancel)
        .show()
        == MessageDialogResult::Ok
}

fn show_info(title: &str, description: &str) {
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title(title)
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
}

fn show_error(title: &str, description: &str) {
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(title)
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_json_sorts_object_keys_recursively() {
        let value = serde_json::json!({
            "z": 1,
            "a": {
                "b": true,
                "a": "x"
            }
        });

        assert_eq!(stable_json(&value), r#"{"a":{"a":"x","b":true},"z":1}"#);
    }

    #[test]
    fn decode_public_key_accepts_server_spki_der_key() {
        let key = decode_public_key("MCowBQYDK2VwAyEAzLdOE7DdxNJqx/5ay6kAERt/9yLnDZxn9yDHFYLDNfE=")
            .expect("decode public key");

        assert_eq!(key.to_bytes().len(), 32);
    }
}
