//! Deterministic production-native sample-source fixtures for runtime validation.

mod audio;
mod manifest;
mod manifest_io;
mod mutation;
mod provision;
#[cfg(any(test, feature = "legacy-controller"))]
mod readiness;
mod specification;
mod topology;

pub use manifest::{FixtureFileManifest, FixtureManifest, FixtureSourceManifest};
pub use mutation::{FixtureMutation, apply_mutation};
pub use provision::{FixtureProvisionRequest, provision, validate};
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) use readiness::wait_for_readiness;
pub use specification::{FIXTURE_VERSION, FixtureName, FixtureProfile};

#[cfg(test)]
mod tests;
