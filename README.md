# stamp-cli

[<img alt="github" src="https://img.shields.io/badge/github-mcmah309/stamp--cli-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/mcmah309/stamp-cli)
[<img alt="crates.io" src="https://img.shields.io/crates/v/stamp-cli.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/stamp-cli)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-stamp--cli-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/stamp-cli)

stamp-cli is a command-line tool for managing and rendering project templates. It allows you to register, list, remove, and render templates from a registry or directly from a source directory.

```console
A cli tool for applying project templates

Usage: stamp [OPTIONS] <COMMAND>

Commands:
  use       Render a template in the registry to a destination directory
  from      Render a template from a source directory to a destination directory
  register  Register a template source directory. All templates within this directory (recursive) will be available
  remove    Remove a registered source directory
  list      List registered templates
  help      Print this message or the help of the given subcommand(s)

Options:
      --debug    Show raw error details, context, and a backtrace
  -h, --help     Print help
  -V, --version  Print version
```

## Conflict resolution

Existing files are left untouched when the incoming content is identical or differs only in leading/trailing whitespace on each line or at the file boundariesl.

`stamp from` and `stamp use` prompt for each conflicting file and show the merge algorithm. Set one flag to apply a strategy to all conflicts:

- `--overwrite-conflicts`: replace with the incoming file.
- `--skip-conflicts`: keep the original file.
- `--merge-overwrite-conflicts`: merge, favoring incoming content on conflicts.
- `--merge-skip-conflicts`: merge, favoring original content on conflicts.

Markdown merges by section; JSON, TOML, and YAML merge by key. Other files and invalid structured files fall back to Patience line diff for text or Myers byte diff for binary data.

## .tera
stamp-cli uses [tera](https://keats.github.io/tera/docs/#templates) for templating. Any file including `.tera` will be treated as a tera template when applying a template through the `use` or `from`
sub commands. e.g. `path/file.tera.json` or `path/file.json.tera`.

Any file name or directory name including a template interpolation (`{{ ... }}`) will also be treated as a template.

## stamp.toml
Add a `stamp.toml` file to a directory to make the directory a valid template. All fields are optional. Example config:
```toml
[meta]
description = "A generic template for devcontainers"
name = "My Template"

# String input
[[questions]]
id = "name"
type = "string" # `bool` also supported
prompt = "What is the project name?"
default = "my-project"

# Select one from a list
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
```console
root@c-nixos:/workspaces/stamp-cli (master)$ stamp register tests/templates/
Source `/workspaces/stamp-cli/tests/templates` registered successfully
root@c-nixos:/workspaces/stamp-cli (master)$ stamp list
Bash - A scaffold for bash scripts with pre-set options and error handling. Plus a cheatsheet.
  /home/henry/templates/bash_script

Axum server - A rust server template built with axum
  /home/henry/templates/axum_server

Devcontainer - A demplate for devcontainers
  /home/henry/templates/devcontainer

Python - A Python project setup
  /home/henry/templates/python-project

root@c-nixos:/workspaces/stamp-cli (master)$ stamp use devcontainer example_crate
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

## Install

Cargo
```bash
cargo install stamp-cli
```
