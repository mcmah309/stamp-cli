use crate::merge::MergePreference;
use dialoguer::{Select, theme::ColorfulTheme};
use eros::bail;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConflictResolution {
    Overwrite,
    Skip,
    Merge(MergePreference),
}

pub fn prompt_conflict(destination: &Path, algorithm: &str) -> eros::Result<ConflictResolution> {
    if !console::Term::stderr().is_term() {
        bail!(
            "Destination `{}` already exists. Conflict resolution requires an interactive terminal. \
             Use --overwrite-conflicts, --skip-conflicts, --merge-overwrite-conflicts, \
             or --merge-skip-conflicts to resolve conflicts non-interactively.",
            destination.display()
        );
    }

    let options = [
        (
            "Overwrite — replace with the incoming file".to_owned(),
            ConflictResolution::Overwrite,
        ),
        (
            "Skip — keep the original file".to_owned(),
            ConflictResolution::Skip,
        ),
        (
            format!("Merge overwrite — {algorithm}, favor incoming"),
            ConflictResolution::Merge(MergePreference::Incoming),
        ),
        (
            format!("Merge skip — {algorithm}, favor original"),
            ConflictResolution::Merge(MergePreference::Original),
        ),
    ];
    let labels: Vec<_> = options.iter().map(|(label, _)| label).collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "File `{}` already exists. How should it be handled?",
            destination.display()
        ))
        .items(&labels)
        .default(1)
        .interact_opt()?;

    match selection {
        Some(index) => Ok(options[index].1),
        None => bail!("Conflict resolution cancelled; no files were written."),
    }
}
