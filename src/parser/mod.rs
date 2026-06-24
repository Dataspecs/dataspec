mod ast;
mod extract;
mod markdown;

pub use ast::Section;
pub use markdown::parse_sections;

use std::path::{Path, PathBuf};

use crate::entities::{Config, Entity, Model, Operation, Template, Test, Transformation};
use crate::error::{ParseError, Result};
use crate::parser::extract::{
    extract_sql, parse_link_label, parse_list_items, parse_props_hashmap, parse_template_usage,
    parse_transformation_body, ParsedTransformationBody,
};

/// Result of parsing a single spec file.
#[derive(Debug, Clone)]
pub struct ParsedSpec {
    pub entity: Entity,
    /// Additional entities derived from the spec (e.g. embedded default transformation).
    pub derived: Vec<Entity>,
}

/// Parse a spec markdown file into typed entities.
pub fn parse_spec(path: impl AsRef<Path>, source: &str) -> Result<ParsedSpec> {
    let path = path.as_ref().to_path_buf();
    let sections = parse_sections(source);
    let root = sections
        .into_iter()
        .next()
        .ok_or_else(|| ParseError::MissingTitle { path: path.clone() })?;

    let entity_type = root
        .child("Type")
        .and_then(|s| s.body_trimmed().lines().next())
        .map(|line| line.trim().to_ascii_lowercase())
        .ok_or_else(|| ParseError::MissingType { path: path.clone() })?;

    let description = section_description(&root);

    match entity_type.as_str() {
        "config" => Ok(ParsedSpec {
            entity: Entity::Config(parse_config(&root)),
            derived: vec![],
        }),
        "model" => parse_model_spec(&root, description),
        "transformation" => Ok(ParsedSpec {
            entity: Entity::Transformation(parse_transformation(&root, description)),
            derived: vec![],
        }),
        "template" => Ok(ParsedSpec {
            entity: Entity::Template(parse_template(&root, description)),
            derived: vec![],
        }),
        "test" => Ok(ParsedSpec {
            entity: Entity::Test(parse_test(&root, description)),
            derived: vec![],
        }),
        "operation" => Ok(ParsedSpec {
            entity: Entity::Operation(parse_operation(&root, description)),
            derived: vec![],
        }),
        other => Err(ParseError::UnknownEntityType {
            path,
            entity_type: other.to_string(),
        }),
    }
}

/// Read and parse a spec file from disk.
pub fn parse_spec_file(path: impl AsRef<Path>) -> Result<ParsedSpec> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path).map_err(|source| ParseError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_spec(path, &source)
}

fn section_description(section: &Section) -> Option<String> {
    let body = section.body_trimmed();
    if body.is_empty() {
        None
    } else {
        Some(body.to_string())
    }
}

fn parse_config(root: &Section) -> Config {
    let type_section = root.child("Type").map(|s| s.body_trimmed()).unwrap_or("");
    let props = parse_props_hashmap(type_section);
    Config { props }
}

fn parse_model_spec(root: &Section, description: Option<String>) -> Result<ParsedSpec> {
    let name = root.title.clone();
    let tags = root
        .child("Tags")
        .map(|s| parse_list_items(s.body_trimmed()))
        .filter(|t| !t.is_empty());

    let default_transformation_link = root
        .child("Default transformation")
        .or_else(|| root.child("Default Transformation"))
        .and_then(|s| parse_link_label(s.body_trimmed()));

    let transformation_section = root
        .child("Transformation")
        .filter(|s| !s.children.is_empty() || !s.body_trimmed().is_empty());

    let body = transformation_section
        .map(parse_transformation_body)
        .unwrap_or_else(|| parse_transformation_body(root));

    let has_embedded_sql = !body.sql_code.is_empty();
    let default_transformation = if let Some(label) = default_transformation_link {
        Some(label)
    } else if has_embedded_sql {
        Some(format!("{name}__default_transformation"))
    } else {
        None
    };

    let model = Model {
        name: name.clone(),
        description,
        tags,
        table_id: None,
        managed: false,
        disabled: false,
        meta: None,
        default_transformation: default_transformation.clone(),
    };

    let derived = if has_embedded_sql || body.has_content() {
        let transformation_name = default_transformation
            .unwrap_or_else(|| format!("{name}__default_transformation"));
        vec![Entity::Transformation(body_to_transformation(
            transformation_name,
            name,
            body,
        ))]
    } else {
        vec![]
    };

    Ok(ParsedSpec {
        entity: Entity::Model(model),
        derived,
    })
}

fn body_to_transformation(
    name: String,
    model: String,
    body: ParsedTransformationBody,
) -> Transformation {
    Transformation {
        name,
        sql_code: body.sql_code,
        model,
        dependent_tables: vec![],
        used_variables: None,
        template: body.template,
        columns: body.columns,
        tests: body.tests,
        pre_runs: body.pre_runs,
        post_runs: body.post_runs,
        init_runs: body.init_runs,
    }
}

fn parse_transformation(root: &Section, _description: Option<String>) -> Transformation {
    let model = root
        .child("Model")
        .and_then(|s| parse_link_label(s.body_trimmed()))
        .expect("transformation requires ## Model link");

    let body = parse_transformation_body(root);
    body_to_transformation(root.title.clone(), model, body)
}

fn parse_template(root: &Section, description: Option<String>) -> Template {
    let transformation = root.child("Transformation");
    let nested_template = transformation
        .and_then(|s| s.child("Template"))
        .and_then(|s| parse_template_usage(s.body_trimmed()));
    let sql_code = transformation
        .and_then(|s| s.child("Code"))
        .and_then(|s| extract_sql(s.body_trimmed()))
        .unwrap_or_default();
    Template {
        name: root.title.clone(),
        description,
        sql_code,
        dependent_tables: vec![],
        used_variables: None,
        template: nested_template,
    }
}

fn parse_test(root: &Section, description: Option<String>) -> Test {
    let transformation = root.child("Transformation");
    let sql_code = transformation
        .and_then(|s| s.child("Code"))
        .and_then(|s| extract_sql(s.body_trimmed()))
        .unwrap_or_default();
    Test {
        name: root.title.clone(),
        description,
        sql_code,
        dependent_tables: vec![],
        used_variables: None,
    }
}

fn parse_operation(root: &Section, description: Option<String>) -> Operation {
    let transformation = root.child("Transformation");
    let template = transformation
        .and_then(|s| s.child("Template"))
        .and_then(|s| parse_template_usage(s.body_trimmed()));
    let sql_code = transformation
        .and_then(|s| s.child("Code"))
        .and_then(|s| extract_sql(s.body_trimmed()))
        .unwrap_or_default();
    let tags = root
        .child("Tags")
        .map(|s| parse_list_items(s.body_trimmed()))
        .filter(|t| !t.is_empty());
    Operation {
        name: root.title.clone(),
        description,
        tags,
        sql_code,
        template,
        dependent_tables: vec![],
        used_variables: None,
    }
}

/// Parse all `.md` spec files under a directory tree.
pub fn parse_spec_dir(dir: impl AsRef<Path>) -> Result<Vec<(PathBuf, Entity)>> {
    let mut entities = Vec::new();
    for entry in walkdir::WalkDir::new(dir.as_ref())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            let parsed = parse_spec_file(path)?;
            entities.push((path.to_path_buf(), parsed.entity));
            for derived in parsed.derived {
                entities.push((path.to_path_buf(), derived));
            }
        }
    }
    entities.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name().cmp(b.1.name())));
    Ok(entities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../specs/data-specs")
            .join(name)
    }

    #[test]
    fn parses_dummy_model() {
        let path = fixture("models/dummy_model.md");
        let parsed = parse_spec_file(&path).expect("parse dummy_model");
        let Entity::Model(model) = parsed.entity else {
            panic!("expected model");
        };
        assert_eq!(model.name, "dummy_model");
        assert_eq!(
            model.tags.as_deref(),
            Some(vec!["vata".to_string(), "vata2".to_string(), "vata3".to_string()].as_slice())
        );
        assert_eq!(
            model.default_transformation.as_deref(),
            Some("dummy_model__default_transformation")
        );
        assert_eq!(parsed.derived.len(), 1);
        let Entity::Transformation(t) = &parsed.derived[0] else {
            panic!("expected derived transformation");
        };
        assert_eq!(t.name, "dummy_model__default_transformation");
        assert_eq!(t.columns.as_ref().map(|c| c.len()), Some(2));
        assert_eq!(t.columns.as_ref().unwrap()[0].name, "dummy1");
        assert!(!t.sql_code.is_empty());
    }

    #[test]
    fn parses_dummy_transformation() {
        let path = fixture("transformations/dummy_model_v1.md");
        let parsed = parse_spec_file(&path).expect("parse transformation");
        let Entity::Transformation(t) = parsed.entity else {
            panic!("expected transformation");
        };
        assert_eq!(t.name, "dummy_model_v1");
        assert_eq!(t.model, "dummy_model");
    }

    #[test]
    fn parses_config() {
        let path = fixture("config/config.md");
        let parsed = parse_spec_file(&path).expect("parse config");
        let Entity::Config(config) = parsed.entity else {
            panic!("expected config");
        };
        assert!(!config.props.is_empty());
        assert_eq!(config.get("environment"), Some("development"));
    }
}
