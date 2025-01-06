# stamp-cli

[<img alt="github" src="https://img.shields.io/badge/github-mcmah309/stamp--cli-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/mcmah309/stamp-cli)
[<img alt="crates.io" src="https://img.shields.io/crates/v/stamp-cli.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/stamp-cli)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-stamp--cli-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/stamp-cli)

stamp-cli is a command-line tool for managing and rendering templates. It allows you to register, list, remove, and render templates from a registry or directly from a source directory.

```console
A cli tool for templates

Usage: stamp <COMMAND>

Commands:
  use       Render a template in the registry to a destination directory
  from      Render a template from as source directory to a destination directory
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
Add a `stamp.yaml` file to a directory to make the directory a valid template
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
## Example
```console
root@c-nixos:/workspaces/stamp-cli (master)$ stamp register tests/templates/ -a
Adding template `axum_server`
Adding template `rust`
Adding template `flutter_rust`
Templates registered successfully
root@c-nixos:/workspaces/stamp-cli (master)$ stamp list
rust:
        path: /workspaces/stamp-cli/tests/templates/devcontainers/rust
axum_server:
        path: /workspaces/stamp-cli/tests/templates/axum_server
flutter_rust:
        path: /workspaces/stamp-cli/tests/templates/devcontainers/flutter_rust
root@c-nixos:/workspaces/stamp-cli (master)$ mkdir example_crate && cd example_crate/
root@c-nixos:/workspaces/stamp-cli/example_crate (master)$ stamp use axum_server .
🎤 crate_name - Name of crate
[]:example_crate
Template rendered successfully to "."
root@c-nixos:/workspaces/stamp-cli/example_crate (master)$ l
total 20K
drwxr-xr-x 3 root root 4.0K Jan  6 09:11 .
drwxr-xr-x 8 root root 4.0K Jan  6 09:10 ..
-rw-r--r-- 1 root root    7 Jan  6 09:11 .gitignore
-rw-r--r-- 1 root root  347 Jan  6 09:11 Cargo.toml
drwxr-xr-x 5 root root 4.0K Jan  6 09:11 src
root@c-nixos:/workspaces/stamp-cli/example_crate (master)$ cat Cargo.toml 
[package]
name = "example_crate"
version = "0.1.0"
edition = "2021"

[lib]
name = "example_crate_lib"
path = "src/lib.rs"

[[bin]]
name = "example_crate"
path = "src/bin/main.rs"

[dependencies]
axum = {version = "0.8.0", features = ["ws"] }
tracing = "0.1"
tracing-subscriber = "0.3"
tokio = { version = "1", features = ["full"] }
```

See [tests/templates/](https://github.com/mcmah309/stamp-cli/tree/master/tests/templates) for more.

