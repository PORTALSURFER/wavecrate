#![deny(missing_docs)]
#![deny(warnings)]

//! Cross-platform command-line entrypoint for deterministic native source fixtures.

use std::{path::PathBuf, str::FromStr};

use wavecrate::native_source_fixture::{
    FixtureMutation, FixtureName, FixtureProfile, FixtureProvisionRequest, apply_mutation,
    provision, validate,
};

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("[wavecrate-fixture][error] {error}");
        std::process::exit(2);
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    let Some(command) = arguments.first().map(String::as_str) else {
        print_usage();
        return Ok(());
    };
    if matches!(command, "help" | "-h" | "--help") {
        print_usage();
        return Ok(());
    }
    let options = Options::parse(&arguments[1..])?;
    match command {
        "provision" => {
            let manifest = provision(&FixtureProvisionRequest {
                config_base: options.config_base,
                fixture: options.fixture,
                profile: options.profile,
                reset: options.reset,
            })?;
            print_manifest(&manifest)
        }
        "validate" => {
            let manifest = validate(&options.config_base, options.fixture, options.profile)?;
            print_manifest(&manifest)
        }
        "mutate" => {
            let mutation = options.mutation.ok_or_else(|| {
                String::from("mutate requires --scenario <mutation>; run --help for names")
            })?;
            apply_mutation(
                options.config_base,
                options.fixture,
                options.profile,
                mutation,
            )?;
            println!(
                "{{\"fixture\":\"{}\",\"mutation\":\"{}\",\"status\":\"applied\"}}",
                options.fixture,
                mutation_name(mutation)
            );
            Ok(())
        }
        _ => Err(format!(
            "unknown command {command:?}; expected provision, validate, or mutate"
        )),
    }
}

struct Options {
    fixture: FixtureName,
    profile: FixtureProfile,
    config_base: PathBuf,
    reset: bool,
    mutation: Option<FixtureMutation>,
}

impl Options {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut fixture = None;
        let mut profile = FixtureProfile::Sandbox;
        let mut config_base = None;
        let mut reset = true;
        let mut mutation = None;
        let mut index = 0;
        while index < arguments.len() {
            match arguments[index].as_str() {
                "--fixture" => {
                    fixture = Some(FixtureName::from_str(value(arguments, index)?)?);
                    index += 2;
                }
                "--profile" => {
                    profile = FixtureProfile::from_str(value(arguments, index)?)?;
                    index += 2;
                }
                "--config-base" => {
                    config_base = Some(PathBuf::from(value(arguments, index)?));
                    index += 2;
                }
                "--scenario" => {
                    mutation = Some(FixtureMutation::from_str(value(arguments, index)?)?);
                    index += 2;
                }
                "--no-reset" => {
                    reset = false;
                    index += 1;
                }
                option => return Err(format!("unknown option {option:?}")),
            }
        }
        Ok(Self {
            fixture: fixture.ok_or_else(|| String::from("missing --fixture <name>"))?,
            profile,
            config_base: config_base.ok_or_else(|| String::from("missing --config-base <path>"))?,
            reset,
            mutation,
        })
    }
}

fn value(arguments: &[String], option_index: usize) -> Result<&str, String> {
    arguments
        .get(option_index + 1)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} requires a value", arguments[option_index]))
}

fn print_manifest(manifest: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(manifest)
            .map_err(|error| format!("serialize fixture result: {error}"))?
    );
    Ok(())
}

fn mutation_name(mutation: FixtureMutation) -> &'static str {
    match mutation {
        FixtureMutation::Create => "create",
        FixtureMutation::SameSizeChange => "same-size-change",
        FixtureMutation::Move => "move",
        FixtureMutation::Delete => "delete",
        FixtureMutation::RootOffline => "root-offline",
        FixtureMutation::RootOnline => "root-online",
        FixtureMutation::Reset => "reset",
    }
}

fn print_usage() {
    println!(
        "Usage: wavecrate-fixture <provision|validate|mutate> --fixture <empty|small-multi-source|large-source> --config-base <path> [--profile <sandbox|automated-tests>] [--no-reset] [--scenario <mutation>]"
    );
}
