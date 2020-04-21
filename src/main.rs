extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate simple_error;

use clap::{crate_authors, crate_version};
use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use simple_error::SimpleError;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::str;

fn main() -> Result<(), SimpleError> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(crate_version!())
        .author(crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("gcloud-configuration-name")
                .help("The configuration name to use for the local gcloud configuration")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::with_name("gcloud-project")
                .help("The ID of the project to create the keyring in in gcloud")
                .index(2)
                .required(true),
        )
        .arg(
            Arg::with_name("gcloud-keyring")
                .help("The name of the keyring in gcloud")
                .index(3)
                .required(true),
        )
        .arg(
            Arg::with_name("gcloud-key")
                .help("The name of the key in the keyring in gcloud")
                .index(4)
                .required(true),
        )
        .get_matches();

    let configuration_name = &matches
        .value_of("gcloud-configuration-name")
        .unwrap()
        .to_string();
    let secret_project = &matches.value_of("gcloud-project").unwrap().to_string();
    let keyring = &matches.value_of("gcloud-keyring").unwrap().to_string();
    let key = &matches.value_of("gcloud-key").unwrap().to_string();

    let active_configuration =
        try_with!(active_configuration(), "Failed to get active configuration");
    let configurations = try_with!(configurations(), "Failed to load configuration config");

    if !configurations.contains(&configuration_name.to_string()) {
        try_with!(
            create_configuration(&configuration_name),
            "Failed to create configuration \"{}\"",
            configuration_name
        );
    }

    if let Some(configuration) = &active_configuration {
        try_with!(
            activate_configuration(configuration.as_str()),
            "Failed to activate configuration \"{}\"",
            configuration_name
        );
    }

    try_with!(
        set_project(secret_project, configuration_name),
        "Failed to set project \"{}\" in configuration \"{}\"",
        secret_project,
        configuration_name
    );

    if !try_with!(
        is_logged_in(configuration_name),
        "Login command failed with configuration \"{}\"",
        configuration_name
    )
    .success()
    {
        try_with!(
            login(configuration_name),
            "Failed to login to configuration \"{}\"",
            configuration_name
        );
    }

    if !try_with!(
        is_cloudkms_service_enabled(configuration_name),
        "Failed check if cloudkms is enabled with config \"{}\"",
        configuration_name
    ) {
        try_with!(
            enable_cloudkms_service(configuration_name),
            "Failed to enable cloudkms service with config \"{}\"",
            configuration_name
        );
    }

    if !try_with!(
        is_keyring_existent(configuration_name, secret_project, keyring),
        "Failed check if keyring \"{}\" in project \"{}\" exists with config \"{}\"",
        keyring,
        secret_project,
        configuration_name
    ) {
        try_with!(
            create_keyring(configuration_name, keyring),
            "Failed create keyring \"{}\" in project \"{}\" exists with config \"{}\"",
            keyring,
            secret_project,
            configuration_name
        );
    }

    if !try_with!(
        is_key_existent(configuration_name, secret_project, keyring, key),
        "Failed check if key \"{}\" in keyring \"{}\" in project \"{}\" exists with config \"{}\"",
        key,
        keyring,
        secret_project,
        configuration_name
    ) {
        try_with!(
            create_key(configuration_name, keyring, key),
            "Failed create key \"{}\" keyring \"{}\" in project \"{}\" exists with config \"{}\"",
            key,
            keyring,
            secret_project,
            configuration_name
        );
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct Configuration<'a> {
    name: &'a str,
}

fn active_configuration() -> Result<Option<String>, SimpleError> {
    let output = Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("list")
        .arg("--filter")
        .arg("is_active:true")
        .arg("--format")
        .arg("json")
        .output()
        .or_else(|e| Err(SimpleError::from(e)))?;
    let output_stdout = str::from_utf8(&output.stdout).or_else(|e| Err(SimpleError::from(e)))?;
    let value: Vec<Configuration> = serde_json::from_str::<Vec<Configuration>>(&output_stdout)
        .or_else(|e| Err(SimpleError::from(e)))?;
    let configurations: Vec<String> = value
        .into_iter()
        .map(|x: Configuration| x.name.to_string())
        .collect();

    match configurations.last() {
        None => Ok(None),
        Some(output) => Ok(Some(output.to_string())),
    }
}

fn configurations() -> Result<Vec<String>, SimpleError> {
    let output = Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .or_else(|e| Err(SimpleError::from(e)))?;
    let output_stdout = str::from_utf8(&output.stdout).or_else(|e| Err(SimpleError::from(e)))?;
    let value: Vec<Configuration> = serde_json::from_str::<Vec<Configuration>>(&output_stdout)
        .or_else(|e| Err(SimpleError::from(e)))?;
    Ok(value
        .into_iter()
        .map(|x: Configuration| x.name.to_string())
        .collect::<Vec<String>>())
}

fn create_configuration(configuration: &str) -> Result<Output, SimpleError> {
    Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("create")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))
}

fn activate_configuration(configuration: &str) -> Result<Output, SimpleError> {
    Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("activate")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))
}

fn set_project(project: &str, configuration: &str) -> Result<Output, SimpleError> {
    Command::new("gcloud")
        .arg("config")
        .arg("set")
        .arg("project")
        .arg(project)
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))
}

fn is_logged_in(configuration: &str) -> Result<ExitStatus, SimpleError> {
    Command::new("gcloud")
        .stdout(Stdio::null())
        .arg("auth")
        .arg("print-identity-token")
        .arg("--configuration")
        .arg(configuration)
        .status()
        .or_else(|e| Err(SimpleError::from(e)))
}

fn login(configuration: &str) -> Result<Output, SimpleError> {
    Command::new("gcloud")
        .arg("auth")
        .arg("login")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))
}

fn is_cloudkms_service_enabled(configuration: &str) -> Result<bool, SimpleError> {
    let output = Command::new("gcloud")
        .arg("services")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--enabled")
        .arg("--filter")
        .arg("name:cloudkms.googleapis.com")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))?;
    let output_stdout = str::from_utf8(&output.stdout).or_else(|e| Err(SimpleError::from(e)))?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(&output_stdout).or_else(|e| Err(SimpleError::from(e)))?;
    Ok(!value.is_empty())
}

fn enable_cloudkms_service(configuration: &str) -> Result<bool, SimpleError> {
    let output = Command::new("gcloud")
        .arg("services")
        .arg("enable")
        .arg("cloudkms.googleapis.com")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))?;
    let output_stdout = str::from_utf8(&output.stdout).or_else(|e| Err(SimpleError::from(e)))?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(&output_stdout).or_else(|e| Err(SimpleError::from(e)))?;
    Ok(!value.is_empty())
}

fn is_keyring_existent(
    configuration: &str,
    project: &str,
    keyring: &str,
) -> Result<bool, SimpleError> {
    let output = Command::new("gcloud")
        .arg("kms")
        .arg("keyrings")
        .arg("list")
        .arg("--location")
        .arg("global")
        .arg("--filter")
        .arg(format!(
            "name:projects/{}/locations/global/keyRings/{}",
            project, keyring
        ))
        .arg("--format")
        .arg("json")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))?;
    let output_stdout = str::from_utf8(&output.stdout).or_else(|e| Err(SimpleError::from(e)))?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(&output_stdout).or_else(|e| Err(SimpleError::from(e)))?;
    Ok(!value.is_empty())
}

fn create_keyring(configuration: &str, keyring: &str) -> Result<Output, SimpleError> {
    Command::new("gcloud")
        .arg("kms")
        .arg("keyrings")
        .arg("create")
        .arg(keyring)
        .arg("--location")
        .arg("global")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))
}

fn is_key_existent(
    configuration: &str,
    project: &str,
    keyring: &str,
    key: &str,
) -> Result<bool, SimpleError> {
    let output = Command::new("gcloud")
        .arg("kms")
        .arg("keys")
        .arg("list")
        .arg("--location")
        .arg("global")
        .arg("--keyring")
        .arg(keyring)
        .arg("--filter")
        .arg(format!(
            "name:projects/{}/locations/global/keyRings/{}/cryptoKeys/{}",
            project, keyring, key
        ))
        .arg("--format")
        .arg("json")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))?;
    let output_stdout = str::from_utf8(&output.stdout).or_else(|e| Err(SimpleError::from(e)))?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(&output_stdout).or_else(|e| Err(SimpleError::from(e)))?;
    Ok(!value.is_empty())
}

fn create_key(configuration: &str, keyring: &str, key: &str) -> Result<Output, SimpleError> {
    Command::new("gcloud")
        .arg("kms")
        .arg("keys")
        .arg("create")
        .arg(key)
        .arg("--location")
        .arg("global")
        .arg("--keyring")
        .arg(keyring)
        .arg("--purpose")
        .arg("encryption")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .or_else(|e| Err(SimpleError::from(e)))
}
