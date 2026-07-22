use std::collections::HashMap;

use crate::entities::{Column, OperationUsage, TemplateUsage, TestUsage};
use crate::parser::ast::Section;

struct LinkRef {
    label: String,
    path: String,
    #[allow(dead_code)]
    anchor: Option<String>,
}

struct PropEntry {
    key: String,
    value: String,
    #[allow(dead_code)]
    description: Option<String>,
}

struct RefWithProps {
    link: LinkRef,
    props: Vec<PropEntry>,
}

/// Parsed transformation body fields shared by models and transformations.
#[derive(Default)]
pub struct ParsedTransformationBody {
    pub columns: Option<Vec<Column>>,
    pub tests: Option<Vec<TestUsage>>,
    pub template: Option<TemplateUsage>,
    pub pre_runs: Option<Vec<OperationUsage>>,
    pub post_runs: Option<Vec<OperationUsage>>,
    pub init_runs: Option<Vec<OperationUsage>>,
    pub sql_code: String,
}

impl ParsedTransformationBody {
    pub fn has_content(&self) -> bool {
        !self.sql_code.is_empty()
            || self.columns.is_some()
            || self.tests.is_some()
            || self.template.is_some()
            || self.pre_runs.is_some()
            || self.post_runs.is_some()
            || self.init_runs.is_some()
    }
}

/// Parse markdown list items (`- item`) from body text.
pub fn parse_list_items(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .map(str::trim)
                .map(str::to_string)
        })
        .collect()
}

/// Parse the first markdown link in body: `[label](path#anchor)`.
fn parse_link(body: &str) -> Option<LinkRef> {
    let start = body.find('[')?;
    let after_open = &body[start + 1..];
    let close = after_open.find("](")?;
    let label = after_open[..close]
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    let after_paren = &after_open[close + 2..];
    let paren_end = after_paren.find(')')?;
    let target = after_paren[..paren_end]
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    let (path, anchor) = target
        .split_once('#')
        .map(|(p, a)| (p.to_string(), Some(a.to_string())))
        .unwrap_or_else(|| (target, None));
    Some(LinkRef {
        label,
        path,
        anchor,
    })
}

/// Parse the label from the first markdown link in body.
pub fn parse_link_label(body: &str) -> Option<String> {
    parse_link(body).map(|link| link.label)
}

fn parse_ref_list(body: &str) -> Vec<RefWithProps> {
    let mut refs = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- ") && !current_lines.is_empty() {
            if let Some(parsed) = parse_ref_block(&current_lines.join("\n")) {
                refs.push(parsed);
            }
            current_lines.clear();
        }
        if trimmed.starts_with("- ") || (!trimmed.is_empty() && !current_lines.is_empty()) {
            current_lines.push(line);
        }
    }
    if !current_lines.is_empty() {
        if let Some(parsed) = parse_ref_block(&current_lines.join("\n")) {
            refs.push(parsed);
        }
    }
    refs
}

fn parse_ref_block(block: &str) -> Option<RefWithProps> {
    let link = parse_link(block)?;
    let props = block
        .find('|')
        .map(|i| parse_props_table(&block[i..]))
        .unwrap_or_default();
    Some(RefWithProps { link, props })
}

/// Parse a markdown table with Key/Value[/Description] columns.
fn parse_props_table(body: &str) -> Vec<PropEntry> {
    let mut rows = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        if cells.is_empty() {
            continue;
        }
        if cells[0].eq_ignore_ascii_case("key") {
            continue;
        }
        if cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':' || ch.is_whitespace()))
        {
            continue;
        }
        if cells.len() >= 2 {
            rows.push(PropEntry {
                key: strip_ticks(cells[0]),
                value: strip_ticks(cells[1]),
                description: cells.get(2).map(|d| strip_ticks(d)),
            });
        }
    }

    rows
}

pub fn parse_props_hashmap(body: &str) -> HashMap<String, String> {
    props_to_hashmap(&parse_props_table(body))
}

fn props_to_hashmap(props: &[PropEntry]) -> HashMap<String, String> {
    props
        .iter()
        .map(|p| (p.key.clone(), p.value.clone()))
        .collect()
}

/// Extract SQL from a fenced code block in section body.
pub fn extract_sql(body: &str) -> Option<String> {
    let start = body.find("```")?;
    let after_fence = &body[start + 3..];
    let content_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
    let rest = &after_fence[content_start..];
    let end = rest.find("```")?;
    Some(rest[..end].trim_end().to_string())
}

pub fn parse_template_usage(body: &str) -> Option<TemplateUsage> {
    let link = parse_link(body)?;
    let table_start = body.find('|').unwrap_or(body.len());
    let props = if table_start < body.len() {
        let map = props_to_hashmap(&parse_props_table(&body[table_start..]));
        if map.is_empty() {
            None
        } else {
            Some(map)
        }
    } else {
        None
    };
    Some(TemplateUsage {
        name: link.label,
        props,
    })
}

fn test_name_from_ref(link: &LinkRef) -> String {
    link.path
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.split('#').next().unwrap_or(segment).to_string())
        .unwrap_or_else(|| link.label.clone())
}

fn refs_to_test_usages(refs: Vec<RefWithProps>) -> Vec<TestUsage> {
    refs.into_iter()
        .map(|r| TestUsage {
            name: test_name_from_ref(&r.link),
            props: {
                let map = props_to_hashmap(&r.props);
                if map.is_empty() {
                    None
                } else {
                    Some(map)
                }
            },
            sql_code: String::new(),
        })
        .collect()
}

fn refs_to_operation_usages(refs: Vec<RefWithProps>) -> Vec<OperationUsage> {
    refs.into_iter()
        .map(|r| OperationUsage {
            name: test_name_from_ref(&r.link),
            props: {
                let map = props_to_hashmap(&r.props);
                if map.is_empty() {
                    None
                } else {
                    Some(map)
                }
            },
            sql_code: String::new(),
        })
        .collect()
}

fn optional_vec<T>(items: Vec<T>) -> Option<Vec<T>> {
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

pub fn parse_columns(section: &Section) -> Vec<Column> {
    section
        .children
        .iter()
        .map(|col_section| {
            let labels = col_section
                .child("Labels")
                .map(|s| parse_list_items(s.body_trimmed()));
            let data_type = col_section
                .child("Type")
                .map(|s| s.body_trimmed().to_string());
            let tests = col_section
                .child("Tests")
                .map(|s| refs_to_test_usages(parse_ref_list(s.body_trimmed())));
            let description = col_section.body_trimmed();
            let description = if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            };
            Column {
                name: col_section.title.clone(),
                description,
                data_type,
                labels,
                tests,
            }
        })
        .collect()
}

pub fn parse_transformation_body(root: &Section) -> ParsedTransformationBody {
    let columns = root
        .child("Columns")
        .map(parse_columns)
        .filter(|cols| !cols.is_empty());
    let tests = root
        .child("Tests")
        .map(|s| refs_to_test_usages(parse_ref_list(s.body_trimmed())));
    let pre_runs = root
        .child("Hooks")
        .and_then(|h| h.child("Pre"))
        .map(|s| refs_to_operation_usages(parse_ref_list(s.body_trimmed())));
    let post_runs = root
        .child("Hooks")
        .and_then(|h| h.child("Post"))
        .map(|s| refs_to_operation_usages(parse_ref_list(s.body_trimmed())));
    let init_runs = root
        .child("Hooks")
        .and_then(|h| h.child("Init"))
        .map(|s| refs_to_operation_usages(parse_ref_list(s.body_trimmed())));
    let template = root
        .child("Template")
        .and_then(|s| parse_template_usage(s.body_trimmed()));
    let sql_code = root
        .child("Code")
        .and_then(|s| extract_sql(s.body_trimmed()))
        .unwrap_or_default();
    ParsedTransformationBody {
        columns,
        tests,
        template,
        pre_runs: optional_vec(pre_runs.unwrap_or_default()),
        post_runs: optional_vec(post_runs.unwrap_or_default()),
        init_runs: optional_vec(init_runs.unwrap_or_default()),
        sql_code,
    }
}

fn strip_ticks(s: &str) -> String {
    s.trim().trim_matches('`').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_link_with_anchor() {
        let link = parse_link("[col](../models/foo#bar)").unwrap();
        assert_eq!(link.label, "col");
        assert_eq!(link.path, "../models/foo");
        assert_eq!(link.anchor.as_deref(), Some("bar"));
    }

    #[test]
    fn parses_multiline_markdown_link() {
        let link = parse_link("- [dummy_test\n](../tests/dummy_test)").unwrap();
        assert_eq!(link.label, "dummy_test");
        assert_eq!(link.path, "../tests/dummy_test");
    }

    #[test]
    fn parses_props_table() {
        let table = "| Key | Value | Description |\n| --- | --- | --- |\n| `a` | `1` | desc |\n";
        let props = parse_props_table(table);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].key, "a");
        assert_eq!(props[0].value, "1");
        assert_eq!(props[0].description.as_deref(), Some("desc"));
    }

    #[test]
    fn parses_tests_section_body_from_dummy_model_v1() {
        let source = std::fs::read_to_string(crate::test_fixtures::fixture(
            "transformations/dummy_model_v1.md",
        ))
        .unwrap();
        let root = &crate::parser::parse_sections(&source)[0];
        let body = root.child("Tests").unwrap().body_trimmed();
        let usages = refs_to_test_usages(parse_ref_list(body));
        assert!(
            usages.iter().any(|usage| usage.name == "dummy_test"),
            "body={body:?}, usages={usages:?}"
        );
    }
}
