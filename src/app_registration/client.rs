use super::{
    ACTIVATION_BASE_URL, APP_ID, MAX_LICENSE_RESPONSE_BYTES, RETRY_COUNT, RegisteredBuild,
    activation::LocalActivation,
    entitlement::{SignedEntitlement, verify_entitlement},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io::Read, time::Duration};

#[derive(Debug, Deserialize)]
pub(super) struct LicenseResponse {
    pub(super) entitlement: Value,
    pub(super) signature: String,
    pub(super) signature_algorithm: String,
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

pub(super) fn renew_or_activate(
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
