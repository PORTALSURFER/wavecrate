use std::{fs, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use super::{
    FixtureName, FixtureProfile, FixtureProvisionRequest,
    audio::{AudioSpec, write_deterministic_wav},
    provision, validate,
};

/// Deterministic mutations supported by native source-system scenarios.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixtureMutation {
    /// Create one new valid WAV under the mutable fixture directory.
    Create,
    /// Replace one WAV with same-length, different deterministic audio.
    SameSizeChange,
    /// Move one WAV to a different nested relative path.
    Move,
    /// Delete one known WAV.
    Delete,
    /// Move the second fixture source root offline.
    RootOffline,
    /// Restore the second fixture source root after `root-offline`.
    RootOnline,
    /// Reconstruct the complete clean baseline.
    Reset,
}

impl FromStr for FixtureMutation {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "create" => Ok(Self::Create),
            "same-size-change" => Ok(Self::SameSizeChange),
            "move" => Ok(Self::Move),
            "delete" => Ok(Self::Delete),
            "root-offline" => Ok(Self::RootOffline),
            "root-online" => Ok(Self::RootOnline),
            "reset" => Ok(Self::Reset),
            _ => Err(format!(
                "unknown fixture mutation {value:?}; expected create, same-size-change, move, delete, root-offline, root-online, or reset"
            )),
        }
    }
}

/// Apply one deterministic source mutation, or reset the fixture to baseline.
pub fn apply_mutation(
    config_base: PathBuf,
    fixture: FixtureName,
    profile: FixtureProfile,
    mutation: FixtureMutation,
) -> Result<(), String> {
    if fixture != FixtureName::SmallMultiSource && mutation != FixtureMutation::Reset {
        return Err(String::from(
            "deterministic mutation scenarios are supported only by small-multi-source",
        ));
    }
    if mutation == FixtureMutation::Reset {
        provision(&FixtureProvisionRequest {
            config_base,
            fixture,
            profile,
            reset: true,
        })?;
        return Ok(());
    }

    let config_base = config_base
        .canonicalize()
        .map_err(|error| format!("resolve fixture config base: {error}"))?;

    let source_beta = config_base
        .join(crate::app_dirs::APP_DIR_NAME)
        .join("fixtures")
        .join(fixture.as_str())
        .join("source-beta");
    let offline = source_beta.with_file_name("source-beta.offline");
    if mutation == FixtureMutation::RootOnline {
        if source_beta.exists() {
            return Err(format!(
                "fixture source is already online: {}",
                source_beta.display()
            ));
        }
        fs::rename(&offline, &source_beta).map_err(|error| {
            format!(
                "restore fixture source {} to {}: {error}",
                offline.display(),
                source_beta.display()
            )
        })?;
        validate(&config_base, fixture, profile)?;
        return Ok(());
    }

    validate(&config_base, fixture, profile)?;
    match mutation {
        FixtureMutation::Create => write_deterministic_wav(
            &source_beta.join("mutable/created.wav"),
            &AudioSpec {
                channels: 1,
                sample_rate: 44_100,
                frames: 5_512,
                seed: 31,
            },
        ),
        FixtureMutation::SameSizeChange => write_deterministic_wav(
            &source_beta.join("mutable/change-me.wav"),
            &AudioSpec {
                channels: 1,
                sample_rate: 44_100,
                frames: 5_512,
                seed: 230,
            },
        ),
        FixtureMutation::Move => {
            let destination = source_beta.join("moved/move-me.wav");
            let parent = destination
                .parent()
                .ok_or_else(|| String::from("fixture move destination has no parent"))?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("create fixture move destination: {error}"))?;
            fs::rename(source_beta.join("mutable/move-me.wav"), &destination)
                .map_err(|error| format!("move fixture file to {}: {error}", destination.display()))
        }
        FixtureMutation::Delete => fs::remove_file(source_beta.join("mutable/delete-me.wav"))
            .map_err(|error| format!("delete fixture file: {error}")),
        FixtureMutation::RootOffline => fs::rename(&source_beta, &offline).map_err(|error| {
            format!(
                "move fixture source {} offline to {}: {error}",
                source_beta.display(),
                offline.display()
            )
        }),
        FixtureMutation::RootOnline | FixtureMutation::Reset => unreachable!(),
    }
}
