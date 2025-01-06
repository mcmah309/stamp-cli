use anyhow::bail;
use clap::{Parser, Subcommand};
use dialoguer::Input;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, process::exit};
use tera::{Context, Tera};

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
    /// Render a template from as source directory to a destination directory
    From {
        /// Path to the source folder
        source: PathBuf,
        /// Path to the destination folder
        destination: PathBuf,
    },
    /// Register templates
    Register {
        /// Path to register templates from
        path: PathBuf,
    },
    /// List registered templates
    List,
}

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    name: String,
    description: Option<String>,
    variables: HashMap<String, VariableConfig>,
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
        Commands::Register { path } => register_templates(path),
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
    let config_contents = fs::read_to_string(&config_path)?;
    let config: TemplateConfig = serde_yaml::from_str(&config_contents)?;

    let mut context = Context::new();

    // user prompt
    for (key, variable) in &config.variables {
        let postfix = variable
            .description
            .as_ref()
            .map(|e| format!(": {e}"))
            .unwrap_or("".to_string());
        let prompt_message = format!("{key}{postfix}");
        let user_value: String = if let Some(ref default) = variable.default {
            Input::<String>::new()
                .with_prompt(format!("🎤 {}", prompt_message))
                .default(default.clone())
                .interact_text()?
        } else {
            Input::new()
                .with_prompt(format!("🎤 {}", prompt_message))
                .interact_text()?
        };
        context.insert(key, &user_value);
    }

    let mut tera = Tera::default();
    tera.autoescape_on(vec![]);
    tera.set_escape_fn(|e| e.to_string());

    for entry in walkdir::WalkDir::new(&template_path) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(&template_path)?;
        let output_path = destination_path.join(relative_path);

        if path.is_file() {
            if path.extension().map_or(false, |ext| ext == "tera") {
                // Render .tera template
                let template_name = relative_path.to_string_lossy();
                tera.add_template_file(&path, Some(&template_name))?;
                let rendered = tera.render(&template_name, &context)?;

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(output_path.with_extension(""), rendered)?;
            } else {
                // Copy other files
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&path, &output_path)?;
            }
        }
    }

    println!("Template rendered successfully to {:?}", destination_path);
    Ok(())
}

fn register_templates(path: PathBuf) -> anyhow::Result<()> {
    let mut registry = load_registry()?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let config_path = path.join("stamp.yaml");
            if config_path.exists() {
                let config_contents = fs::read_to_string(&config_path)?;
                let config: TemplateConfig = serde_yaml::from_str(&config_contents)?;
                let info = RegistryInfo {
                    description: config.description,
                    path: path.to_string_lossy().to_string(),
                };
                registry.templates.insert(config.name.clone(), info);
            }
        }
    }

    save_registry(&registry)?;
    println!("Templates registered successfully.");
    Ok(())
}

fn list_templates() -> anyhow::Result<()> {
    let registry = load_registry()?;

    for (name, info) in registry.templates {
        let RegistryInfo { description, path } = info;
        if let Some(description) = description {
            println!("{}:\n\tdescription:{}\n\tpath:{}", name, description, path);
        } else {
            println!("{}:\n\tpath:{}", name, path);
        }
    }

    Ok(())
}

fn load_registry() -> anyhow::Result<Registry> {
    let registry_path = get_registry_path()?;
    if let Ok(contents) = fs::read_to_string(&registry_path) {
        let registry: Registry = serde_json::from_str(&contents)?;
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
