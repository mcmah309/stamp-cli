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

See [tests/templates/](https://github.com/mcmah309/stamp-cli/tree/master/tests/templates) for more.