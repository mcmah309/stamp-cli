use super::*;
use std::fs;

fn fixture(case: &str, name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/templates/merges")
        .join(case)
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn assert_case(case: &str, extension: &str, algorithm: &str) {
    for (preference, expected) in [
        (MergePreference::Incoming, "merge-overwrite"),
        (MergePreference::Original, "merge-skip"),
    ] {
        let prepared = PreparedMerge::new(
            Path::new(&format!("file.{extension}")),
            fixture(case, &format!("original.{extension}")),
            fixture(case, &format!("incoming.{extension}")),
        );
        assert_eq!(prepared.algorithm(), algorithm, "{case}");
        let actual = prepared.merge(preference).unwrap();
        let expected = fixture(case, &format!("{expected}.{extension}"));
        match algorithm {
            "JSON keys" => {
                // Compare values and key order, allowing insignificant whitespace.
                let actual: serde_json::Value = serde_json::from_slice(&actual).unwrap();
                let expected: serde_json::Value = serde_json::from_slice(&expected).unwrap();
                assert_eq!(
                    actual.to_string(),
                    expected.to_string(),
                    "{case}: {preference:?}"
                );
            }
            "TOML keys" => {
                let actual = String::from_utf8(actual).unwrap();
                let expected = String::from_utf8(expected).unwrap();
                assert_eq!(
                    toml::from_str::<toml::Value>(&actual).unwrap(),
                    toml::from_str::<toml::Value>(&expected).unwrap(),
                    "{case}: {preference:?}"
                );
                for comment in expected
                    .lines()
                    .filter_map(|line| line.split_once('#').map(|(_, comment)| comment))
                {
                    assert!(
                        actual.contains(&format!("#{comment}")),
                        "{case}: lost comment {comment}"
                    );
                }
            }
            "YAML keys" => {
                assert_eq!(
                    parse_yaml(std::str::from_utf8(&actual).unwrap()).unwrap(),
                    parse_yaml(std::str::from_utf8(&expected).unwrap()).unwrap(),
                    "{case}: {preference:?}"
                );
            }
            _ => assert_eq!(actual, expected, "{case}: {preference:?}"),
        }
    }
}

#[test]
fn json_merges_nested_keys_and_resolves_values_arrays_and_type_changes() {
    assert_case("json/nested", "json", "JSON keys");
}

#[test]
fn json_preserves_original_key_order_and_appends_new_keys() {
    assert_case("json/key-order", "json", "JSON keys");
}

#[test]
fn toml_merges_nested_tables_and_inline_tables_preserving_untouched_comments() {
    assert_case("toml/nested", "toml", "TOML keys");
}

#[test]
fn toml_merges_across_inline_and_standard_table_representations() {
    for case in ["toml/inline-to-table", "toml/table-to-inline"] {
        assert_case(case, "toml", "TOML keys");
    }
}

#[test]
fn toml_arrays_and_arrays_of_tables_are_whole_values() {
    assert_case("toml/arrays", "toml", "TOML keys");
}

#[test]
fn yaml_merges_nested_keys_and_resolves_sequences_and_type_changes() {
    assert_case("yaml/nested", "yaml", "YAML keys");
}

#[test]
fn yaml_resolves_anchor_merge_keys_before_merging() {
    assert_case("yaml/anchors", "yml", "YAML keys");
}

#[test]
fn yaml_merges_matching_tags_and_treats_different_tags_as_conflicts() {
    for case in ["yaml/matching-tags", "yaml/different-tags"] {
        assert_case(case, "yaml", "YAML keys");
    }
}

#[test]
fn handler_names_reflect_custom_handlers_and_fallbacks() {
    for (case, name, extension, algorithm) in [
        ("markdown/shared", "README.MD", "md", "Markdown sections"),
        (
            "markdown/shared",
            "README.markdown",
            "md",
            "Markdown sections",
        ),
        ("json/nested", "file.JSON", "json", "JSON keys"),
        ("toml/nested", "file.TOML", "toml", "TOML keys"),
        ("yaml/anchors", "file.YML", "yml", "YAML keys"),
        ("text/blocks", "file.txt", "txt", "Patience line diff"),
        ("text/blocks", "file", "txt", "Patience line diff"),
    ] {
        let prepared = PreparedMerge::new(
            Path::new(name),
            fixture(case, &format!("original.{extension}")),
            fixture(case, &format!("incoming.{extension}")),
        );
        assert_eq!(prepared.algorithm(), algorithm, "{name}");
    }
    for (case, extension) in [
        ("fallback/invalid-json", "json"),
        ("fallback/invalid-toml", "toml"),
        ("fallback/invalid-yaml", "yaml"),
        ("fallback/multiple-yaml-documents", "yaml"),
    ] {
        assert_case(case, extension, "Patience line diff");
    }
}

#[test]
fn text_diff_keeps_independent_blocks_and_resolves_competing_edits() {
    assert_case("text/blocks", "txt", "Patience line diff");
}

#[test]
fn byte_diff_retains_non_utf8_and_obeys_the_preference() {
    assert_case("binary/replacement", "bin", "Myers byte diff");
}

#[test]
fn fallback_diff_preserves_empty_content_crlf_unicode_and_final_newlines() {
    for case in [
        "text/empty",
        "text/empty-original",
        "text/empty-incoming",
        "text/crlf-unicode",
    ] {
        assert_case(case, "txt", "Patience line diff");
    }
}

#[test]
fn markdown_shared_bodies_obey_preference_and_unique_sections_survive() {
    assert_case("markdown/shared", "md", "Markdown sections");
}

#[test]
fn markdown_matches_repeated_heading_titles_within_their_parent() {
    assert_case("markdown/parents", "md", "Markdown sections");
}

#[test]
fn markdown_matches_duplicate_sibling_headings_by_occurrence() {
    assert_case("markdown/duplicates", "md", "Markdown sections");
}

#[test]
fn markdown_recognizes_setext_and_formatted_headings() {
    assert_case("markdown/setext", "md", "Markdown sections");
}

#[test]
fn markdown_ignores_heading_syntax_inside_code_quotes_lists_and_html() {
    assert_case("markdown/code-containers", "md", "Markdown sections");
}

#[test]
fn markdown_preserves_both_distinct_documents_as_unique_sections() {
    assert_case("markdown/distinct", "md", "Markdown sections");
}

#[test]
fn markdown_preserves_existing_order_when_incoming_sections_are_reordered() {
    assert_case("markdown/reordered", "md", "Markdown sections");
}

#[test]
fn markdown_separates_appended_sections_when_the_original_has_no_final_newline() {
    assert_case("markdown/no-final-newline", "md", "Markdown sections");
}

#[test]
fn markdown_handles_preambles_empty_files_and_documents_without_headings() {
    for case in [
        "markdown/prose",
        "markdown/preamble",
        "markdown/empty",
        "markdown/empty-original",
        "markdown/empty-incoming",
        "markdown/empty-body",
    ] {
        assert_case(case, "md", "Markdown sections");
    }
}

#[test]
fn markdown_preserves_crlf_unicode_and_identical_documents() {
    assert_case("markdown/crlf-unicode", "md", "Markdown sections");
    for preference in [MergePreference::Incoming, MergePreference::Original] {
        let content = fixture("markdown/crlf-unicode", "original.md");
        let prepared = PreparedMerge::new(Path::new("README.md"), content.clone(), content.clone());
        assert_eq!(prepared.merge(preference).unwrap(), content);
    }
}
