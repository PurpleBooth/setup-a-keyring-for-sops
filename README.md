# Setup a Keyring in GCloud for SOPS

This sets up the keyring and project and everything needed to do things
in SOPS

## Usage

``` bash
setup-a-keyring-for-sops dev
Billie Thompson <billie+setup-a-keyring-for-sops@purplebooth.co.uk>
Create a key for use with SOPS in gcloud.

USAGE:
    setup-a-keyring-for-sops <gcloud-configuration-name> <gcloud-project> <gcloud-keyring> <gcloud-key>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <gcloud-configuration-name>    The configuration name to use for the local gcloud configuration
    <gcloud-project>               The ID of the project to create the keyring in in gcloud
    <gcloud-keyring>               The name of the keyring in gcloud
    <gcloud-key>                   The name of the key in the keyring in gcloud

```

## Installing

First tap my homebrew repo

``` shell
brew tap PurpleBooth/repo
```

Next install the binary

``` shell
brew install PurpleBooth/repo/setup-a-keyring-for-sops
```

You can also download the [latest
release](https://github.com/PurpleBooth/setup-a-keyring-for-sops/releases/latest)
and run it.

## License

[CC0](LICENSE.md) - Public Domain
