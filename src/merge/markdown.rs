use super::MergePreference;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::collections::HashMap;

struct Heading {
    level: u8,
    title: String,
    start: usize,
    end: usize,
}

struct Section<'a> {
    level: u8,
    title: String,
    heading: &'a str,
    body: &'a str,
    children: Vec<Section<'a>>,
}

/// Match headings within their parent sections. Shared section bodies are
/// conflicts; unique sections and subsections are retained from both documents.
pub(super) fn merge(original: &str, incoming: &str, preference: MergePreference) -> String {
    if original == incoming {
        return original.to_owned();
    }
    let mut original = parse(original);
    merge_section(&mut original, parse(incoming), preference);
    let mut result = String::new();
    render(&original, &mut result);
    result
}

fn parse(content: &str) -> Section<'_> {
    let mut headings = Vec::new();
    let mut current: Option<Heading> = None;
    let mut depth = 0;
    // The parser recognizes ATX and Setext headings and excludes headings
    // inside fenced code, blockquotes, lists, and HTML blocks.
    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0
                    && let Tag::Heading { level, .. } = tag
                {
                    current = Some(Heading {
                        level: level as u8,
                        title: String::new(),
                        start: range.start,
                        end: range.end,
                    });
                }
                depth += 1;
            }
            Event::End(tag) => {
                depth -= 1;
                if matches!(tag, TagEnd::Heading(_))
                    && depth == 0
                    && let Some(mut heading) = current.take()
                {
                    heading.end = range.end;
                    headings.push(heading);
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = &mut current {
                    heading.title.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = &mut current {
                    heading.title.push(' ');
                }
            }
            _ => {}
        }
    }
    let mut root = Section {
        level: 0,
        title: String::new(),
        heading: "",
        body: &content[..headings
            .first()
            .map_or(content.len(), |heading| heading.start)],
        children: Vec::new(),
    };
    let mut index = 0;
    while index < headings.len() {
        root.children
            .push(parse_section(content, &headings, &mut index));
    }
    root
}

fn parse_section<'a>(content: &'a str, headings: &[Heading], index: &mut usize) -> Section<'a> {
    let heading = &headings[*index];
    *index += 1;
    let body_end = headings
        .get(*index)
        .map_or(content.len(), |next| next.start);
    let mut section = Section {
        level: heading.level,
        title: heading.title.clone(),
        heading: &content[heading.start..heading.end],
        body: &content[heading.end..body_end],
        children: Vec::new(),
    };
    while *index < headings.len() && headings[*index].level > section.level {
        section
            .children
            .push(parse_section(content, headings, index));
    }
    section
}

fn section_keys(sections: &[Section<'_>]) -> Vec<(u8, String, usize)> {
    let mut occurrences = HashMap::new();
    sections
        .iter()
        .map(|section| {
            let key = (section.level, section.title.clone());
            let count = occurrences.entry(key.clone()).or_insert(0);
            let result = (key.0, key.1, *count);
            *count += 1;
            result
        })
        .collect()
}

fn merge_section<'a>(
    original: &mut Section<'a>,
    incoming: Section<'a>,
    preference: MergePreference,
) {
    if preference == MergePreference::Incoming {
        original.heading = incoming.heading;
    }
    if original.body.trim().is_empty()
        || (preference == MergePreference::Incoming && !incoming.body.trim().is_empty())
    {
        original.body = incoming.body;
    }
    let mut original_keys = section_keys(&original.children);
    let incoming_keys = section_keys(&incoming.children);
    let mut cursor = 0;
    for (key, section) in incoming_keys.into_iter().zip(incoming.children) {
        if let Some(index) = original_keys.iter().position(|existing| *existing == key) {
            merge_section(&mut original.children[index], section, preference);
            cursor = cursor.max(index + 1);
        } else {
            original_keys.insert(cursor, key);
            original.children.insert(cursor, section);
            cursor += 1;
        }
    }
}

fn render(section: &Section<'_>, output: &mut String) {
    if !section.heading.is_empty()
        && !output.is_empty()
        && !output.ends_with("\n\n")
        && !output.ends_with("\r\n\r\n")
    {
        let newline = if output.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        if !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(newline);
    }
    output.push_str(section.heading);
    output.push_str(section.body);
    for child in &section.children {
        render(child, output);
    }
}
