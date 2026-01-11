use clap::{Parser, Subcommand};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use directories::ProjectDirs;
use eros::{bail, Context};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf, process::exit};
use tera::Tera;

#[derive(Parser)]
#[command(name = "yard", author = "Henry McMahon", version = "0.1", about =  "A cli tool for applying project templates", long_about = None)]
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
    /// Register a template source directory. All templates within this directory (recursive) will be available.
    Register {
        /// Path to the source directory to register
        path: PathBuf,
    },
    /// Remove a registered source directory
    Remove {
        /// Path to the source directory to remove
        path: PathBuf,
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

#[derive(Debug, Deserialize, Serialize, Default)]
struct Registry {
    sources: Vec<PathBuf>,
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
        Commands::Register { path } => register_source(path),
        Commands::Remove { path } => remove_source(path),
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
    let templates = find_templates(&registry.sources);

    let mut matches: Vec<&FoundTemplate> = Vec::new();

    let query_path = PathBuf::from(&template_name);

    for template in &templates {
        if template.name == template_name {
            matches.push(template);
            continue;
        }

        if template.path.ends_with(&query_path) {
            matches.push(template);
            continue;
        }
    }

    matches.sort_by_key(|t| &t.path);
    matches.dedup_by_key(|t| &t.path);

    if matches.is_empty() {
        bail!("Template '{}' not found in registry", template_name)
    } else if matches.len() > 1 {
        eprintln!("Ambiguous template match for '{}':", template_name);
        for m in matches {
            eprintln!(" - {} ({})", m.name, m.path.to_string_lossy());
        }
        bail!("Please provide a more specific path or name.");
    }

    let selected = matches[0];
    render_template(selected.path.clone(), destination_path, conflict_strategy)
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
            }
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
            }
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
                    let selected_ids: Vec<&String> =
                        selections.iter().map(|&idx| &choices[idx].id).collect();
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

fn register_source(path: PathBuf) -> eros::Result<()> {
    let mut registry = load_registry()?;
    let canon_path = fs::canonicalize(&path)
        .with_context(|| format!("Could not find path `{}`", path.to_string_lossy()))?;

    if !canon_path.is_dir() {
        bail!("Path must be a directory");
    }

    if registry.sources.contains(&canon_path) {
        println!(
            "Source `{}` already registered",
            canon_path.to_string_lossy()
        );
    } else {
        registry.sources.push(canon_path.clone());
        save_registry(&registry)?;
        println!(
            "Source `{}` registered successfully",
            canon_path.to_string_lossy()
        );
    }

    Ok(())
}

fn list_templates() -> eros::Result<()> {
    let registry = load_registry()?;

    if registry.sources.is_empty() {
        println!("No sources registered");
        return Ok(());
    }

    let templates = find_templates(&registry.sources);

    if templates.is_empty() {
        println!("No templates found in registered sources");
        return Ok(());
    }

    for template in templates {
        print!("{}", style(&template.name).bold().cyan());

        if let Some(desc) = template.description {
            print!(" - {}", style(desc).italic());
        }
        println!();

        println!("  {}", style(template.path.to_string_lossy()).dim());
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
        Ok(Registry::default())
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

fn remove_source(path: PathBuf) -> eros::Result<()> {
    let mut registry = load_registry()?;
    let canon_path = if path.exists() {
        fs::canonicalize(&path)?
    } else {
        path
    };

    if let Some(index) = registry.sources.iter().position(|r| *r == canon_path) {
        registry.sources.remove(index);
        save_registry(&registry)?;
        println!(
            "Source `{}` removed successfully",
            canon_path.to_string_lossy()
        );
    } else {
        bail!(
            "Source `{}` not found in registry",
            canon_path.to_string_lossy()
        );
    }

    Ok(())
}

struct FoundTemplate {
    path: PathBuf,
    name: String,
    description: Option<String>,
}

fn find_templates(sources: &[PathBuf]) -> Vec<FoundTemplate> {
    let mut templates = Vec::new();
    // Used to prevent recursing into already found templates
    let mut excluded_paths: HashSet<PathBuf> = HashSet::new();

    for source in sources {
        let walker = WalkBuilder::new(source)
            .max_depth(Some(4))
            .standard_filters(true)
            .build();

        for result in walker {
            match result {
                Ok(entry) => {
                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        let path = entry.path().to_path_buf();

                        // Skip if ancestor is already a template
                        if excluded_paths
                            .iter()
                            .any(|excluded| path.starts_with(excluded) && path != *excluded)
                        {
                            continue;
                        }

                        let config_path = path.join("stamp.toml");
                        if config_path.exists() {
                            // Found a template
                            excluded_paths.insert(path.clone());

                            // Parse name/desc
                            let name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let mut description = None;
                            let mut display_name = name.clone();

                            if let Ok(contents) = fs::read_to_string(&config_path) {
                                if let Ok(config) = toml::from_str::<TemplateConfig>(&contents) {
                                    if let Some(n) = config.meta.name {
                                        display_name = n;
                                    }
                                    description = config.meta.description;
                                }
                            }

                            templates.push(FoundTemplate {
                                path,
                                name: display_name,
                                description,
                            });
                        }
                    }
                }
                Err(err) => {
                    eros::traced!("Error walking directory: {}", err);
                }
            }
        }
    }

    templates
}
