use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use eros::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::exit,
};
use tera::Tera;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use console::style;

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
        #[clap(default_value = ".")]
        destination: PathBuf,
        /// Overwrite any conflicting files
        #[clap(long, group = "conflict_strategy")]
        overwrite_conflicts: bool,
        /// Skip any conflicting files
        #[clap(long, group = "conflict_strategy")]
        skip_conflicts: bool,
    },
    /// Render a template from a source directory to a destination directory
    From {
        /// Path to the source folder
        source: PathBuf,
        /// Path to the destination folder
        destination: PathBuf,
        /// Overwrite any conflicting files
        #[clap(long, group = "conflict_strategy")]
        overwrite_conflicts: bool,
        /// Skip any conflicting files
        #[clap(long, group = "conflict_strategy")]
        skip_conflicts: bool,
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
    #[serde(default)]
    meta: MetaConfig,
    #[serde(default)]
    questions: Vec<Question>,
}

#[derive(Debug, Deserialize, Default)]
struct MetaConfig {
    description: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Question {
    id: String,
    #[serde(rename = "type")]
    kind: QuestionType,
    prompt: String,
    #[serde(default)]
    default: Option<toml::Value>,
    #[serde(default)]
    options: Option<Vec<String>>,
    #[serde(default)]
    choices: Option<Vec<MultiChoice>>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum QuestionType {
    String,
    Select,
    MultiSelect,
    Bool,
}

#[derive(Debug, Deserialize)]
struct MultiChoice {
    id: String,
    prompt: String,
    #[serde(default)]
    default: bool,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConflictStrategy {
    Fail,
    Overwrite,
    Skip,
}

fn main() -> eros::Result<()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Use {
            name,
            destination,
            overwrite_conflicts,
            skip_conflicts,
        } => {
            let strategy = if overwrite_conflicts {
                ConflictStrategy::Overwrite
            } else if skip_conflicts {
                ConflictStrategy::Skip
            } else {
                ConflictStrategy::Fail
            };
            render_registered_template(name, destination, strategy)
        }
        Commands::From {
            source,
            destination,
            overwrite_conflicts,
            skip_conflicts,
        } => {
            let strategy = if overwrite_conflicts {
                ConflictStrategy::Overwrite
            } else if skip_conflicts {
                ConflictStrategy::Skip
            } else {
                ConflictStrategy::Fail
            };
            render_template(source, destination, strategy)
        }
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
    conflict_strategy: ConflictStrategy,
) -> eros::Result<()> {
    let registry = load_registry()?;
    if let Some(info) = registry.templates.get(&template_name) {
        render_template(PathBuf::from(&info.path), destination_path, conflict_strategy)
    } else {
        bail!("Template '{}' not found in registry", template_name)
    }
}

fn render_template(
    template_path: PathBuf,
    destination_path: PathBuf,
    conflict_strategy: ConflictStrategy,
) -> eros::Result<()> {
    // Load the template configuration
    let config_path = template_path.join("stamp.toml");
    let config_contents = fs::read_to_string(&config_path)
        .with_context(|| format!("could not read `{}`", config_path.to_string_lossy()))?;
    let config: TemplateConfig = toml::from_str(&config_contents).with_context(|| {
        format!(
            "Template config from `{}` is not valid",
            config_path.to_string_lossy()
        )
    })?;

    // Validate config
    let mut validation_errors = Vec::new();
    for question in &config.questions {
        if question.options.is_some() && question.choices.is_some() {
            validation_errors.push(format!(
                "Question '{}' cannot have both 'options' and 'choices'",
                question.id
            ));
        }

        match question.kind {
            QuestionType::Select => {
                if question.options.is_none() {
                    validation_errors.push(format!(
                        "Question '{}' of type 'select' must have 'options'",
                        question.id
                    ));
                }
                if question.choices.is_some() {
                    validation_errors.push(format!(
                        "Question '{}' of type 'select' cannot have 'choices'",
                        question.id
                    ));
                }
            }
            QuestionType::MultiSelect => {
                if question.choices.is_none() {
                    validation_errors.push(format!(
                        "Question '{}' of type 'multi-select' must have 'choices'",
                        question.id
                    ));
                }
                if question.options.is_some() {
                    validation_errors.push(format!(
                        "Question '{}' of type 'multi-select' cannot have 'options'",
                        question.id
                    ));
                }
            }
            QuestionType::String | QuestionType::Bool => {
                if question.options.is_some() {
                    validation_errors.push(format!(
                        "Question '{}' of type '{:?}' cannot have 'options'",
                        question.id, question.kind
                    ));
                }
                if question.choices.is_some() {
                    validation_errors.push(format!(
                        "Question '{}' of type '{:?}' cannot have 'choices'",
                        question.id, question.kind
                    ));
                }
            }
        }
    }

    if !validation_errors.is_empty() {
        eprintln!("Invalid template configuration:");
        for error in validation_errors {
            eprintln!(" - {}", error);
        }
        bail!("Template configuration validation failed");
    }

    if conflict_strategy == ConflictStrategy::Fail {
        let mut early_conflicts = Vec::new();
        for entry in walkdir::WalkDir::new(&template_path) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if path.file_name().is_some_and(|n| n == "stamp.toml") {
                    continue;
                }

                let relative = path.strip_prefix(&template_path)?;
                let relative_str = relative.to_string_lossy();

                if relative_str.contains("{{") {
                   // skip interpolation since these will be replaced and we don't know what the output will look like
                   continue; 
                }
                let mut output_path = destination_path.join(relative);
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let is_tera = file_name.ends_with(".tera") || file_name.contains(".tera.");
                    if is_tera {
                        let new_name = output_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .replace(".tera", "");
                        output_path.set_file_name(new_name);
                    }

                    if output_path.exists() {
                        early_conflicts.push(output_path);
                    }
            }
        }

        if !early_conflicts.is_empty() {
            eprintln!("Conflicting files found:");
            for conflict in early_conflicts {
                eprintln!(" - {}", conflict.to_string_lossy());
            }
            bail!("Destination files already exist. Use --overwrite-conflicts or --skip-conflicts to resolve.");
        }
    }

    let mut context = tera::Context::new();

    // user prompt
    let total_questions = config.questions.len();
    for (i, question) in config.questions.iter().enumerate() {
        let step = i + 1;
        let prompt = format!("[{}/{}] {}", step, total_questions, question.prompt);
        let theme = ColorfulTheme::default();

        match question.kind {
            QuestionType::String => {
                let default_val = question
                    .default
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let mut input = Input::<String>::with_theme(&theme);
                input = input.with_prompt(&prompt);
                
                if let Some(default) = default_val {
                    input = input.default(default);
                }
                
                let value = input.interact()?;
                context.insert(&question.id, &value);
            },
            QuestionType::Bool => {
                let default_val = question
                    .default
                    .as_ref()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let value = Confirm::with_theme(&theme)
                    .with_prompt(&prompt)
                    .default(default_val)
                    .interact()?;
                context.insert(&question.id, &value);
            }
            QuestionType::Select => {
                if let Some(options) = &question.options {
                    let default_idx = question
                        .default
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .and_then(|d| options.iter().position(|r| r == d))
                        .unwrap_or(0);
                    
                    let selection = Select::with_theme(&theme)
                        .with_prompt(&prompt)
                        .default(default_idx)
                        .items(options)
                        .interact()?;
                    
                    context.insert(&question.id, &options[selection]);
                }
            },
            QuestionType::MultiSelect => {
                if let Some(choices) = &question.choices {
                    let defaults: Vec<bool> = choices.iter().map(|c| c.default).collect();
                    let items: Vec<&String> = choices.iter().map(|c| &c.prompt).collect();
                    
                    let selections = MultiSelect::with_theme(&theme)
                        .with_prompt(&prompt)
                        .items(&items)
                        .defaults(&defaults)
                        .interact()?;
                    
                    // Add boolean variables for each choice ID
                    for (idx, choice) in choices.iter().enumerate() {
                        let is_selected = selections.contains(&idx);
                        context.insert(&choice.id, &is_selected);
                    }
                    
                    // Also add a list of selected IDs to the main question ID
                    let selected_ids: Vec<&String> = selections.iter().map(|&idx| &choices[idx].id).collect();
                    context.insert(&question.id, &selected_ids);
                }
            }
        }
    }

    let mut tera = Tera::default();
    tera.autoescape_on(vec![]);
    tera.set_escape_fn(|e| e.to_string());

    struct FileAction {
        source: PathBuf,
        destination: PathBuf,
        is_tera: bool,
    }

    let mut actions: Vec<FileAction> = Vec::new();

    for entry in walkdir::WalkDir::new(&template_path) {
        let entry = entry?;
        let path_in_template = entry.path();

        if path_in_template.is_file() {
            if path_in_template
                .file_name()
                .is_some_and(|name| name == "stamp.toml")
            {
                continue;
            }
        
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
                eros::traced!(
                    "Failed to render path component `{}` of `{}`",
                    component_failed,
                    output_path
                )
            })?;

            let file_name = path_in_template
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let is_tera = file_name.ends_with(".tera") || file_name.contains(".tera.");

            let mut final_output_path = output_path;
            if is_tera {
                let new_name = final_output_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace(".tera", "");
                final_output_path.set_file_name(new_name);
            }

            actions.push(FileAction {
                source: path_in_template.to_path_buf(),
                destination: final_output_path,
                is_tera,
            });
        }
    }

    match conflict_strategy {
        ConflictStrategy::Fail => {
            let mut conflicts = Vec::new();
            for action in &actions {
                if action.destination.exists() {
                    conflicts.push(action.destination.clone());
                }
            }
            if !conflicts.is_empty() {
                eprintln!("Conflicting files found:");
                for conflict in conflicts {
                    eprintln!(" - {}", conflict.to_string_lossy());
                }
                bail!("Destination files already exist. Use --overwrite-conflicts or --skip-conflicts to resolve.");
            }
        }
        ConflictStrategy::Skip => {
            actions.retain(|action| !action.destination.exists());
        }
        ConflictStrategy::Overwrite => {
            // Do nothing, just proceed to overwrite
        }
    }

    for action in actions {
        if let Some(parent) = action.destination.parent() {
            fs::create_dir_all(parent)?;
        }

        if action.is_tera {
            // Render .tera template
            let tera_template = fs::read_to_string(&action.source)?;
            let rendered = tera.render_str(&tera_template, &context)?;
            fs::write(action.destination, rendered)?;
        } else {
            // Copy other files
            fs::copy(&action.source, &action.destination)?;
        }
    }

    println!("Template rendered successfully to {:?}", destination_path);
    Ok(())
}

fn register_templates(path: PathBuf, all: bool, overwrite: bool) -> eros::Result<()> {
    let mut registry = load_registry()?;
    let mut added = 0;
    let mut add_to_registry_fn = |path: &Path| -> eros::Result<()> {
        let config_path = path.join("stamp.toml");
        if config_path.exists() {
            let config_contents = fs::read_to_string(&config_path)?;
            let config: TemplateConfig =
                toml::from_str(&config_contents).with_context(|| {
                    format!(
                        "Template config from `{}` is not valid",
                        config_path.to_string_lossy()
                    )
                })?;
            let info = RegistryInfo {
                description: config.meta.description,
                path: path.canonicalize()?.to_string_lossy().to_string(),
            };
            let name = match config.meta.name {
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

fn list_templates() -> eros::Result<()> {
    let registry = load_registry()?;

    if registry.templates.is_empty() {
        println!("No templates registered");
        return Ok(());
    }

    for (name, info) in registry.templates {
        let RegistryInfo { description, path } = info;
        
        print!("{}", style(&name).bold().cyan());
        
        if let Some(desc) = description {
            print!(" - {}", style(desc).italic());
        }
        println!();
        
        println!("  {}", style(path).dim());
        println!(); 
    }

    Ok(())
}

fn load_registry() -> eros::Result<Registry> {
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

fn get_registry_path() -> eros::Result<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "mcmah309", "stamp") {
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("template_registry.json"))
    } else {
        bail!("Could not determine configuration directory")
    }
}

fn save_registry(registry: &Registry) -> eros::Result<()> {
    let registry_path = get_registry_path()?;
    let contents = serde_json::to_string_pretty(registry)?;
    fs::write(registry_path, contents)?;
    Ok(())
}

fn remove_template(names: Vec<String>, all: bool) -> eros::Result<()> {
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
