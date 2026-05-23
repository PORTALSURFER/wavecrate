use super::{APP_ID, RegisteredBuild, client::LicenseResponse};
use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct SignedEntitlement {
    pub(super) entitlement: Value,
    pub(super) signature: String,
    pub(super) signature_algorithm: String,
}

pub(super) fn verify_entitlement(
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

pub(super) fn activation_success_message(entitlement: &SignedEntitlement) -> String {
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
