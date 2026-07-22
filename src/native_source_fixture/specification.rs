use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// Version of the deterministic native source-fixture contract.
pub const FIXTURE_VERSION: u32 = 1;
/// Stable deterministic seed used by every version-one fixture.
pub const FIXTURE_SEED: u64 = 0x5741_5645_4352_4154;

/// Canonical production-native fixture names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixtureName {
    /// No configured sources, for onboarding and empty-state validation.
    Empty,
    /// Two small configured sources for routine source-system and visual QA.
    SmallMultiSource,
    /// Explicit high-cardinality source fixture for scalability validation.
    LargeSource,
}

impl FixtureName {
    /// Return the stable command-line and manifest name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::SmallMultiSource => "small-multi-source",
            Self::LargeSource => "large-source",
        }
    }
}

impl fmt::Display for FixtureName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for FixtureName {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "empty" => Ok(Self::Empty),
            "small-multi-source" => Ok(Self::SmallMultiSource),
            "large-source" => Ok(Self::LargeSource),
            _ => Err(format!(
                "unknown fixture {value:?}; expected empty, small-multi-source, or large-source"
            )),
        }
    }
}

/// Allowed non-live persistence profiles for fixture provisioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixtureProfile {
    /// Manual and signed-app sandbox profile.
    Sandbox,
    /// Automated native GUI/runtime validation profile.
    AutomatedTests,
}

impl FixtureProfile {
    /// Return the stable persistence profile name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::AutomatedTests => "automated-tests",
        }
    }
}

impl fmt::Display for FixtureProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for FixtureProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "sandbox" => Ok(Self::Sandbox),
            "automated-tests" | "automated" => Ok(Self::AutomatedTests),
            "live" => Err(String::from(
                "fixture provisioning refuses the live persistence profile",
            )),
            _ => Err(format!(
                "unsupported fixture profile {value:?}; expected sandbox or automated-tests"
            )),
        }
    }
}
