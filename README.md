# stamp-cli

[<img alt="github" src="https://img.shields.io/badge/github-mcmah309/stamp--cli-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/mcmah309/stamp-cli)
[<img alt="crates.io" src="https://img.shields.io/crates/v/stamp-cli.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/stamp-cli)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-stamp--cli-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/stamp-cli)

stamp-cli is a command-line tool for managing and rendering project templates. It allows you to register, list, remove, and render templates from a registry or directly from a source directory.

```console
A cli tool for templates

Usage: stamp <COMMAND>

Commands:
  use       Render a template in the registry to a destination directory
  from      Render a template from a source directory to a destination directory
  register  Register templates to the registry
  remove    Remove registered templates
  list      List registered templates
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## .tera
stamp-cli uses [tera](https://keats.github.io/tera/docs/) for templating. Any file with a `.tera`
suffix will be treated as a tera template when applying a template through the `use` or `from`
sub commands.

## stamp.yaml
Add a `stamp.yaml` file to a directory to make the directory a valid template. All fields are optional. Example config:
```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/mcmah309/stamp-cli/master/src/schemas/stamp-schema.json
name: Name of template
description: Description of template
variables:
  template_variable1: # Variable name in template
    description: Description of template variable 
    default: Default value of template variable
  template_variable2:
```
## Usage Example
From [tests/templates/axum_server](https://github.com/mcmah309/stamp-cli/tree/master/tests/templates/axum_server)
```console
root@c-nixos:/workspaces/stamp-cli (master)$ stamp register tests/templates/ -a
Adding template `axum_server`
Adding template `rust`
Adding template `flutter_rust`
Templates registered successfully
root@c-nixos:/workspaces/stamp-cli (master)$ stamp list
axum_server:
        description: An axum server project
        path: /workspaces/stamp-cli/tests/templates/axum_server
flutter_rust:
        path: /workspaces/stamp-cli/tests/templates/devcontainers/flutter_rust
rust:
        path: /workspaces/stamp-cli/tests/templates/devcontainers/rust
root@c-nixos:/workspaces/stamp-cli (master)$ stamp use axum_server example_crate
🎤 crate_name - Name of crate
[]:stamp_poc
Template rendered successfully to "example_crate"
root@c-nixos:/workspaces/stamp-cli (master)$ l example_crate
total 24K
drwxr-xr-x 4 root root 4.0K Jan 10 22:45 .
drwxr-xr-x 8 root root 4.0K Jan 10 22:45 ..
-rw-r--r-- 1 root root    7 Jan 10 22:45 .gitignore
-rw-r--r-- 1 root root  335 Jan 10 22:45 Cargo.toml
drwxr-xr-x 5 root root 4.0K Jan 10 22:45 src
drwxr-xr-x 2 root root 4.0K Jan 10 22:45 stamp_poc
root@c-nixos:/workspaces/stamp-cli (master)$ cat example_crate/Cargo.toml
[package]
name = "stamp_poc"
version = "0.1.0"
edition = "2021"

[lib]
name = "stamp_poc_lib"
path = "src/lib.rs"

[[bin]]
name = "stamp_poc"
path = "src/bin/main.rs"

[dependencies]
axum = {version = "0.8.0", features = ["ws"] }
tracing = "0.1"
tracing-subscriber = "0.3"
tokio = { version = "1", features = ["full"] }
```

See [tests/templates/](https://github.com/mcmah309/stamp-cli/tree/master/tests/templates) for more.

