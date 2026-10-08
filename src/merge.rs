use eros::Context;
use similar::{Algorithm, DiffTag, capture_diff_slices};
use std::path::Path;

mod markdown;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MergePreference {
    Incoming,
    Original,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MergeFormat {
    Markdown,
    Json,
    Toml,
    Yaml,
}

impl MergeFormat {
    pub fn for_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "md" | "markdown" => Some(Self::Markdown),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            "yaml" | "yml" => Some(Self::Yaml),
            _ => None,
        }
    }
}

/// Select a format-aware handler when both files parse; otherwise use a diff.
/// Templates must already have been rendered before choosing the handler.
pub enum PreparedMerge {
    Markdown(String, String),
    Json(serde_json::Value, serde_json::Value),
    Toml(Box<toml_edit::DocumentMut>, Box<toml_edit::DocumentMut>),
    Yaml(serde_yaml_ng::Value, serde_yaml_ng::Value),
    TextDiff(String, String),
    ByteDiff(Vec<u8>, Vec<u8>),
}

impl PreparedMerge {
    pub fn new(path: &Path, original: Vec<u8>, incoming: Vec<u8>) -> Self {
        match (String::from_utf8(original), String::from_utf8(incoming)) {
            (Ok(original), Ok(incoming)) => {
                if let Some(format) = MergeFormat::for_path(path)
                    && let Ok(prepared) = Self::parse(format, &original, &incoming)
                {
                    return prepared;
                }
                Self::TextDiff(original, incoming)
            }
            (original, incoming) => Self::ByteDiff(
                original.map_or_else(|error| error.into_bytes(), |text| text.into_bytes()),
                incoming.map_or_else(|error| error.into_bytes(), |text| text.into_bytes()),
            ),
        }
    }

    fn parse(format: MergeFormat, original: &str, incoming: &str) -> eros::Result<Self> {
        Ok(match format {
            MergeFormat::Markdown => Self::Markdown(original.to_owned(), incoming.to_owned()),
            MergeFormat::Json => Self::Json(
                serde_json::from_str(original).context("Original JSON is invalid")?,
                serde_json::from_str(incoming).context("Incoming JSON is invalid")?,
            ),
            MergeFormat::Toml => Self::Toml(
                Box::new(
                    original
                        .parse::<toml_edit::DocumentMut>()
                        .context("Original TOML is invalid")?,
                ),
                Box::new(
                    incoming
                        .parse::<toml_edit::DocumentMut>()
                        .context("Incoming TOML is invalid")?,
                ),
            ),
            MergeFormat::Yaml => Self::Yaml(
                parse_yaml(original).context("Original YAML is invalid")?,
                parse_yaml(incoming).context("Incoming YAML is invalid")?,
            ),
        })
    }

    pub fn algorithm(&self) -> &'static str {
        match self {
            Self::Markdown(..) => "Markdown sections",
            Self::Json(..) => "JSON keys",
            Self::Toml(..) => "TOML keys",
            Self::Yaml(..) => "YAML keys",
            Self::TextDiff(..) => "Patience line diff",
            Self::ByteDiff(..) => "Myers byte diff",
        }
    }

    pub fn merge(self, preference: MergePreference) -> eros::Result<Vec<u8>> {
        let text = match self {
            Self::Markdown(original, incoming) => markdown::merge(&original, &incoming, preference),
            Self::Json(mut original, incoming) => {
                merge_json(&mut original, incoming, preference);
                format!("{}\n", serde_json::to_string_pretty(&original)?)
            }
            Self::Toml(mut original, incoming) => {
                merge_toml(original.as_item_mut(), incoming.as_item(), preference);
                original.to_string()
            }
            Self::Yaml(mut original, incoming) => {
                merge_yaml(&mut original, incoming, preference);
                serde_yaml_ng::to_string(&original)?
            }
            Self::TextDiff(original, incoming) => {
                let original: Vec<_> = original.split_inclusive('\n').collect();
                let incoming: Vec<_> = incoming.split_inclusive('\n').collect();
                merge_diff(&original, &incoming, Algorithm::Patience, preference).concat()
            }
            Self::ByteDiff(original, incoming) => {
                return Ok(merge_diff(
                    &original,
                    &incoming,
                    Algorithm::Myers,
                    preference,
                ));
            }
        };
        Ok(text.into_bytes())
    }
}

fn parse_yaml(text: &str) -> eros::Result<serde_yaml_ng::Value> {
    let mut value: serde_yaml_ng::Value = serde_yaml_ng::from_str(text)?;
    value.apply_merge()?;
    Ok(value)
}

fn merge_diff<T: Clone + Eq + std::hash::Hash>(
    original: &[T],
    incoming: &[T],
    algorithm: Algorithm,
    preference: MergePreference,
) -> Vec<T> {
    let mut merged = Vec::new();
    for operation in capture_diff_slices(algorithm, original, incoming) {
        let values = match operation.tag() {
            DiffTag::Equal | DiffTag::Delete => &original[operation.old_range()],
            DiffTag::Insert => &incoming[operation.new_range()],
            DiffTag::Replace => match preference {
                MergePreference::Incoming => &incoming[operation.new_range()],
                MergePreference::Original => &original[operation.old_range()],
            },
        };
        merged.extend_from_slice(values);
    }
    merged
}

fn merge_json(
    original: &mut serde_json::Value,
    incoming: serde_json::Value,
    preference: MergePreference,
) {
    match (original, incoming) {
        (serde_json::Value::Object(original), serde_json::Value::Object(incoming)) => {
            for (key, value) in incoming {
                match original.get_mut(&key) {
                    Some(existing) => merge_json(existing, value, preference),
                    None => {
                        original.insert(key, value);
                    }
                }
            }
        }
        (original, incoming) if preference == MergePreference::Incoming => *original = incoming,
        _ => {}
    }
}

fn merge_toml(
    original: &mut toml_edit::Item,
    incoming: &toml_edit::Item,
    preference: MergePreference,
) {
    if let (Some(original), Some(incoming)) =
        (original.as_table_like_mut(), incoming.as_table_like())
    {
        for (key, value) in incoming.iter() {
            match original.get_mut(key) {
                Some(existing) => merge_toml(existing, value, preference),
                None => {
                    original.insert(key, value.clone());
                }
            }
        }
    } else if preference == MergePreference::Incoming {
        *original = incoming.clone();
    }
}

fn merge_yaml(
    original: &mut serde_yaml_ng::Value,
    incoming: serde_yaml_ng::Value,
    preference: MergePreference,
) {
    match (original, incoming) {
        (serde_yaml_ng::Value::Mapping(original), serde_yaml_ng::Value::Mapping(incoming)) => {
            for (key, value) in incoming {
                match original.get_mut(&key) {
                    Some(existing) => merge_yaml(existing, value, preference),
                    None => {
                        original.insert(key, value);
                    }
                }
            }
        }
        (serde_yaml_ng::Value::Tagged(original), serde_yaml_ng::Value::Tagged(incoming))
            if original.tag == incoming.tag =>
        {
            merge_yaml(&mut original.value, incoming.value, preference);
        }
        (original, incoming) if preference == MergePreference::Incoming => *original = incoming,
        _ => {}
    }
}

#[cfg(test)]
mod tests;
