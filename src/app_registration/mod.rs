//! Registered-build access gate for release builds.

mod activation;
mod client;
mod dialogs;
mod entitlement;

use activation::{load_or_create_activation, save_activation};
use client::renew_or_activate;
use dialogs::{confirm_activation_request, show_error, show_info, show_update_required_if_needed};
use entitlement::activation_success_message;

const ACTIVATION_BASE_URL: &str = "https://portalsurfer.org/wavecrate/api/v1";
const DOWNLOAD_URL: &str = "https://portalsurfer.org/wavecrate/releases/";
const APP_ID: &str = "wavecrate";
const MAX_LICENSE_RESPONSE_BYTES: usize = 32 * 1024;
const RETRY_COUNT: usize = 2;

const BUILD_ID: Option<&str> = option_env!("WAVECRATE_BUILD_ID");
const BUILD_SIGNATURE: Option<&str> = option_env!("WAVECRATE_BUILD_SIGNATURE");
const SIGNING_PUBLIC_KEY_B64: Option<&str> = option_env!("WAVECRATE_SIGNING_PUBLIC_KEY_B64");

/// Ensure this release build has an active server registration before launch.
pub(crate) fn ensure_registration() -> Result<(), String> {
    let build = registered_build()?;
    let mut activation = load_or_create_activation()?;
    match renew_or_activate(build, &activation, false) {
        Ok(entitlement) => {
            activation.last_entitlement = Some(entitlement);
            save_activation(&activation)
        }
        Err(first_error) => {
            if show_update_required_if_needed(build.build_id, &first_error) {
                return Err(first_error);
            }
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
                    save_activation(&activation)
                }
                Err(err) => {
                    show_error("Wavecrate access", &err);
                    Err(err)
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RegisteredBuild {
    pub(super) build_id: &'static str,
    pub(super) build_signature: &'static str,
    pub(super) public_key_b64: &'static str,
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
