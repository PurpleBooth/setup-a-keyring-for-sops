extern crate serde;
extern crate serde_json;

use clap::{crate_authors, crate_version};
use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::str;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(crate_version!())
        .author(crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("gcloud-configuration-name")
                .help("The configuration name to use for the local gcloud configuration")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::new("gcloud-project")
                .help("The ID of the project to create the keyring in in gcloud")
                .index(2)
                .required(true),
        )
        .arg(
            Arg::new("gcloud-keyring")
                .help("The name of the keyring in gcloud")
                .index(3)
                .required(true),
        )
        .arg(
            Arg::new("gcloud-key")
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

    let active_configuration = active_configuration()?;
    let configurations = configurations()?;

    if !configurations.contains(configuration_name) {
        create_configuration(configuration_name)?;
    }

    if let Some(configuration) = &active_configuration {
        activate_configuration(configuration.as_str())?;
    }

    set_project(secret_project, configuration_name)?;

    if !is_logged_in(configuration_name)?.success() {
        login(configuration_name)?;
    }

    if !is_cloudkms_service_enabled(configuration_name)? {
        enable_cloudkms_service(configuration_name)?;
    }

    if !is_keyring_existent(configuration_name, secret_project, keyring)? {
        create_keyring(configuration_name, keyring)?;
    }

    if !is_key_existent(configuration_name, secret_project, keyring, key)? {
        create_key(configuration_name, keyring, key)?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct Configuration<'a> {
    name: &'a str,
}

fn active_configuration() -> Result<Option<String>> {
    let output = Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("list")
        .arg("--filter")
        .arg("is_active:true")
        .arg("--format")
        .arg("json")
        .output()
        .map_err(Box::<dyn Error>::from)?;
    let output_stdout = str::from_utf8(&output.stdout).map_err(Box::<dyn Error>::from)?;
    let value: Vec<Configuration> = serde_json::from_str::<Vec<Configuration>>(output_stdout)
        .map_err(Box::<dyn Error>::from)?;
    let configurations: Vec<String> = value
        .into_iter()
        .map(|x: Configuration| x.name.to_string())
        .collect();

    match configurations.last() {
        None => Ok(None),
        Some(output) => Ok(Some(output.to_string())),
    }
}

fn configurations() -> Result<Vec<String>> {
    let output = Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .map_err(Box::<dyn Error>::from)?;
    let output_stdout = str::from_utf8(&output.stdout).map_err(Box::<dyn Error>::from)?;
    let value: Vec<Configuration> = serde_json::from_str::<Vec<Configuration>>(output_stdout)
        .map_err(Box::<dyn Error>::from)?;
    Ok(value
        .into_iter()
        .map(|x: Configuration| x.name.to_string())
        .collect::<Vec<String>>())
}

fn create_configuration(configuration: &str) -> Result<Output> {
    Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("create")
        .arg(configuration)
        .output()
        .map_err(Box::<dyn Error>::from)
}

fn activate_configuration(configuration: &str) -> Result<Output> {
    Command::new("gcloud")
        .arg("config")
        .arg("configurations")
        .arg("activate")
        .arg(configuration)
        .output()
        .map_err(Box::<dyn Error>::from)
}

fn set_project(project: &str, configuration: &str) -> Result<Output> {
    Command::new("gcloud")
        .arg("config")
        .arg("set")
        .arg("project")
        .arg(project)
        .arg("--configuration")
        .arg(configuration)
        .output()
        .map_err(Box::<dyn Error>::from)
}

fn is_logged_in(configuration: &str) -> Result<ExitStatus> {
    Command::new("gcloud")
        .stdout(Stdio::null())
        .arg("auth")
        .arg("print-identity-token")
        .arg("--configuration")
        .arg(configuration)
        .status()
        .map_err(Box::<dyn Error>::from)
}

fn login(configuration: &str) -> Result<Output> {
    Command::new("gcloud")
        .arg("auth")
        .arg("login")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .map_err(Box::<dyn Error>::from)
}

fn is_cloudkms_service_enabled(configuration: &str) -> Result<bool> {
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
        .map_err(Box::<dyn Error>::from)?;
    let output_stdout = str::from_utf8(&output.stdout).map_err(Box::<dyn Error>::from)?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(output_stdout).map_err(Box::<dyn Error>::from)?;
    Ok(!value.is_empty())
}

fn enable_cloudkms_service(configuration: &str) -> Result<bool> {
    let output = Command::new("gcloud")
        .arg("services")
        .arg("enable")
        .arg("cloudkms.googleapis.com")
        .arg("--configuration")
        .arg(configuration)
        .output()
        .map_err(Box::<dyn Error>::from)?;
    let output_stdout = str::from_utf8(&output.stdout).map_err(Box::<dyn Error>::from)?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(output_stdout).map_err(Box::<dyn Error>::from)?;
    Ok(!value.is_empty())
}

fn is_keyring_existent(configuration: &str, project: &str, keyring: &str) -> Result<bool> {
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
        .map_err(Box::<dyn Error>::from)?;
    let output_stdout = str::from_utf8(&output.stdout).map_err(Box::<dyn Error>::from)?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(output_stdout).map_err(Box::<dyn Error>::from)?;
    Ok(!value.is_empty())
}

fn create_keyring(configuration: &str, keyring: &str) -> Result<Output> {
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
        .map_err(Box::<dyn Error>::from)
}

fn is_key_existent(configuration: &str, project: &str, keyring: &str, key: &str) -> Result<bool> {
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
        .map_err(Box::<dyn Error>::from)?;
    let output_stdout = str::from_utf8(&output.stdout).map_err(Box::<dyn Error>::from)?;
    let value: Vec<Configuration> =
        serde_json::from_str::<Vec<_>>(output_stdout).map_err(Box::<dyn Error>::from)?;
    Ok(!value.is_empty())
}

fn create_key(configuration: &str, keyring: &str, key: &str) -> Result<Output> {
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
        .map_err(Box::<dyn Error>::from)
}
