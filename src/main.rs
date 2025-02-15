use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::exit,
};
use tera::Tera;

#[derive(Parser)]
#[command(name = "yard", author = "Henry McMahon", version = "0.1", about =  "A cli tool for templates", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a template in the registry to a destination directory
    Use {
        /// The template name in the registry
        name: String,
        /// Path to the destination folder
        destination: PathBuf,
    },
    /// Render a template from a source directory to a destination directory
    From {
        /// Path to the source folder
        source: PathBuf,
        /// Path to the destination folder
        destination: PathBuf,
    },
    /// Register templates to the registry
    Register {
        /// Recursively register all templates in the directory
        #[clap(long, short, default_value = "false")]
        all: bool,
        /// Overwrite existing templates if names conflict with existing
        #[clap(long, short, default_value = "false")]
        overwrite: bool,
        /// Path to register templates from
        path: PathBuf,
    },
    /// Remove registered templates
    Remove {
        /// The template names in the registry to remove
        #[clap(long, short)]
        name: Vec<String>,
        /// Removes all registered templates
        #[clap(long, short)]
        all: bool,
    },
    /// List registered templates
    List,
}

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    name: Option<String>,
    description: Option<String>,
    variables: Option<HashMap<String, VariableConfig>>,
}

#[derive(Debug, Deserialize)]
struct VariableConfig {
    description: Option<String>,
    default: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Registry {
    templates: HashMap<String, RegistryInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RegistryInfo {
    description: Option<String>,
    path: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Use { name, destination } => render_registered_template(name, destination),
        Commands::From {
            source,
            destination,
        } => render_template(source, destination),
        Commands::Register {
            path,
            all,
            overwrite,
        } => register_templates(path, all, overwrite),
        Commands::Remove { name, all } => remove_template(name, all),
        Commands::List => list_templates(),
    };

    if let Err(error) = result {
        eprintln!("Oops something went wrong.\n");
        eprintln!("{:?}", error);
        exit(1);
    };

    Ok(())
}

fn render_registered_template(
    template_name: String,
    destination_path: PathBuf,
) -> anyhow::Result<()> {
    let registry = load_registry()?;
    if let Some(info) = registry.templates.get(&template_name) {
        render_template(PathBuf::from(&info.path), destination_path)
    } else {
        bail!("Template '{}' not found in registry", template_name)
    }
}

fn render_template(template_path: PathBuf, destination_path: PathBuf) -> anyhow::Result<()> {
    // Load the template configuration
    let config_path = template_path.join("stamp.yaml");
    let config_contents = fs::read_to_string(&config_path)
        .with_context(|| format!("could not read `{}`", config_path.to_string_lossy()))?;
    let config: TemplateConfig = serde_yaml::from_str(&config_contents).with_context(|| {
        format!(
            "Template config from `{}` is not valid",
            config_path.to_string_lossy()
        )
    })?;

    let mut context = tera::Context::new();

    // user prompt
    io::stdout().flush().unwrap();
    for (key, variable) in &config.variables.unwrap_or_default() {
        let postfix = variable
            .description
            .as_ref()
            .map(|e| format!(" - {e}"))
            .unwrap_or("".to_string());
        let prompt_message = format!("{key}{postfix}");
        let mut user_value: String = String::new();
        println!("🎤 {prompt_message}");
        if let Some(default) = &variable.default {
            print!("[{default}]:")
        } else {
            print!("[]:")
        }
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut user_value).unwrap();
        user_value = user_value.trim().to_owned();
        if user_value.is_empty() {
            if let Some(default) = &variable.default {
                user_value = default.clone();
            }
        }
        context.insert(key, &user_value);
    }

    let mut tera = Tera::default();
    tera.autoescape_on(vec![]);
    tera.set_escape_fn(|e| e.to_string());

    for entry in walkdir::WalkDir::new(&template_path) {
        let entry = entry?;
        let path_in_template = entry.path();
        let relative_path_in_template = path_in_template.strip_prefix(&template_path)?;
        let output_path_original = destination_path.join(relative_path_in_template);
        // Treat each path component as a template
        let output_path: Result<PathBuf, String> = output_path_original
            .components()
            .map(|e| {
                let str_part = e.as_os_str().to_string_lossy();
                let processed_part = tera.render_str(&str_part, &context);
                processed_part.map_err(|_| str_part.to_string())
            })
            .try_fold(PathBuf::new(), |acc, part| Ok(acc.join(&part?)));
        let output_path = output_path.map_err(|component_failed| {
            let output_path = output_path_original.to_string_lossy();
            anyhow::anyhow!(
                "Failed to render path component `{component_failed}` of `{output_path}`"
            )
        })?;

        if path_in_template.is_file() {
            if path_in_template
                .file_name()
                .is_some_and(|name| name == "stamp.yaml")
            {
                continue;
            }
            if path_in_template
                .extension()
                .map_or(false, |ext| ext == "tera")
            {
                // Render .tera template
                let tera_template = fs::read_to_string(path_in_template)?;
                let rendered = tera.render_str(&tera_template, &context)?;

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(output_path.with_extension(""), rendered)?;
            } else {
                // Copy other files
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&path_in_template, &output_path)?;
            }
        }
    }

    println!("Template rendered successfully to {:?}", destination_path);
    Ok(())
}

fn register_templates(path: PathBuf, all: bool, overwrite: bool) -> anyhow::Result<()> {
    let mut registry = load_registry()?;
    let mut added = 0;
    let mut add_to_registry_fn = |path: &Path| -> anyhow::Result<()> {
        let config_path = path.join("stamp.yaml");
        if config_path.exists() {
            let config_contents = fs::read_to_string(&config_path)?;
            let config: TemplateConfig =
                serde_yaml::from_str(&config_contents).with_context(|| {
                    format!(
                        "Template config from `{}` is not valid",
                        config_path.to_string_lossy()
                    )
                })?;
            let info = RegistryInfo {
                description: config.description,
                path: path.canonicalize()?.to_string_lossy().to_string(),
            };
            let name = match config.name {
                Some(value) => value,
                None => path
                    .components()
                    .last()
                    .unwrap()
                    .as_os_str()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            };
            if registry.templates.contains_key(&name) {
                if overwrite {
                    println!("Overwriting template `{}`", name);
                    registry.templates.insert(name, info);
                    added += 1;
                } else {
                    println!("Template `{}` already registered - not adding", name);
                }
            } else {
                println!("Adding template `{}`", name);
                registry.templates.insert(name, info);
                added += 1;
            }
        }
        Ok(())
    };

    if !path.exists() {
        bail!("Register path does not exist");
    }
    if path.is_file() {
        bail!("Register path must be a directory");
    }
    if all {
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                add_to_registry_fn(path)?;
            }
        }
    } else {
        add_to_registry_fn(&path)?;
    }
    if added == 0 {
        assert!(!registry.templates.is_empty());
        println!("No templates added");
        return Ok(());
    }
    save_registry(&registry)?;
    println!("Templates registered successfully");
    Ok(())
}

fn list_templates() -> anyhow::Result<()> {
    let registry = load_registry()?;

    if registry.templates.is_empty() {
        println!("No templates registered");
    }

    for (name, info) in registry.templates {
        let RegistryInfo { description, path } = info;
        if let Some(description) = description {
            println!(
                "{}:\n\tdescription: {}\n\tpath: {}",
                name, description, path
            );
        } else {
            println!("{}:\n\tpath: {}", name, path);
        }
    }

    Ok(())
}

fn load_registry() -> anyhow::Result<Registry> {
    let registry_path = get_registry_path()?;
    if let Ok(contents) = fs::read_to_string(&registry_path) {
        let registry: Registry = serde_json::from_str(&contents).with_context(|| {
            format!(
                "Registry from `{}` is not valid",
                registry_path.to_string_lossy()
            )
        })?;
        Ok(registry)
    } else {
        Ok(Registry {
            templates: HashMap::new(),
        })
    }
}

fn get_registry_path() -> anyhow::Result<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "mcmah309", "stamp") {
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("template_registry.json"))
    } else {
        bail!("Could not determine configuration directory")
    }
}

fn save_registry(registry: &Registry) -> anyhow::Result<()> {
    let registry_path = get_registry_path()?;
    let contents = serde_json::to_string_pretty(registry)?;
    fs::write(registry_path, contents)?;
    Ok(())
}

fn remove_template(names: Vec<String>, all: bool) -> anyhow::Result<()> {
    let mut registry = load_registry()?;
    if all {
        registry.templates.clear();
        save_registry(&registry)?;
        println!("All templates removed successfully");
        return Ok(());
    }
    for name in names {
        if registry.templates.remove(&name).is_some() {
            save_registry(&registry)?;
            println!("Template `{}` removed successfully", name);
        } else {
            bail!("Template `{}` not found in registry", name)
        }
    }
    Ok(())
}
