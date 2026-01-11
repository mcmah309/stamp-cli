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

## stamp.toml
Add a `stamp.toml` file to a directory to make the directory a valid template. All fields are optional. Example config:
```toml
[meta]
description = "A generic template for devcontainers"
name = "My Template"

# String input
[[questions]]
id = "name"
type = "string"
prompt = "What is the project name?"
default = "my-project"

# Selection from a list
[[questions]]
id = "toolchain"
type = "select"
prompt = "Choose a toolchain:"
options = ["stable", "beta", "nightly"]
default = "stable"

# Multiple selection
[[questions]]
id = "features"
type = "multi-select"
prompt = "Select optional components:"
choices = [
  { id = "rust_support", prompt = "Rust LSP", default = true },
  { id = "sh_support",   prompt = "Shell LSP", default = false },
  { id = "c_support",    prompt = "C LSP",     default = false }
]
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
devcontainer
  /home/henry/templates/devcontainer

axum_server
  /home/henry/templates/axum_server

python-project
  /home/henry/templates/python-project

bash_script
  /home/henry/templates/bash_script

bun
  /home/henry/templates/bun
root@c-nixos:/workspaces/stamp-cli (master)$ stamp use axum_server example_crate
✔ [1/3] Container Name · rust
✔ [2/3] Base Image · rust:latest
? [3/3] Which features would you like to include? ›
⬚ Shell LSP support
⬚ Rust LSP and tools
⬚ C LSP and tools
⬚ Zig LSP and tools
⬚ Python LSP and tools
⬚ Flutter/Dart LSP and tools
⬚ Dioxus support
⬚ Web dev (HTML/CSS/Tailwind)
⬚ USB devices support
⬚ XDG portal support
Template rendered successfully to "example_crate"
```

See [tests/templates/](https://github.com/mcmah309/stamp-cli/tree/master/tests/templates) for more.

## Install
Cargo
```bash
cargo install stamp-cli
```