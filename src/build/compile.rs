use std::collections::HashMap;

use crate::context::render_compile_deferred;
use crate::entities::{
    Config, Entity, Operation, OperationUsage, Template, TemplateUsage, Test, Transformation,
};
use crate::error::{ParseError, Result};
use crate::sql::get_dependent_tables;

/// Compile-time rendering: inline templates and substitute `{{props__*}}`.
pub fn compile_entities(entities: &mut [(std::path::PathBuf, Entity)], config: &Config) -> Result<()> {
    let initial_templates = index_templates(entities);
    let mut compiled_templates = initial_templates.clone();

    for _ in 0..initial_templates.len().max(1) {
        let mut changed = false;
        for (name, template) in &initial_templates {
            let mut compiled = template.clone();
            compile_template_entity(&mut compiled, &compiled_templates, &config.props)?;
            if compiled_templates
                .get(name)
                .is_none_or(|existing| existing.sql_code != compiled.sql_code)
            {
                compiled_templates.insert(name.clone(), compiled);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for (_, entity) in entities.iter_mut() {
        if let Entity::Template(t) = entity {
            if let Some(compiled) = compiled_templates.get(&t.name) {
                *t = compiled.clone();
            }
        }
    }

    for (_, entity) in entities.iter_mut() {
        match entity {
            Entity::Transformation(t) => compile_transformation(t, &compiled_templates, &config.props)?,
            Entity::Operation(o) => compile_operation(o, &compiled_templates, &config.props)?,
            Entity::Test(t) => compile_test(t, &config.props)?,
            Entity::Config(_) | Entity::Model(_) | Entity::Template(_) => {}
        }
    }

    Ok(())
}

fn index_templates(entities: &[(std::path::PathBuf, Entity)]) -> HashMap<String, Template> {
    entities
        .iter()
        .filter_map(|(_, entity)| match entity {
            Entity::Template(t) => Some((t.name.clone(), t.clone())),
            _ => None,
        })
        .collect()
}

fn compile_transformation(
    transformation: &mut Transformation,
    templates: &HashMap<String, Template>,
    config_props: &HashMap<String, String>,
) -> Result<()> {
    let entity_name = transformation.name.clone();
    transformation.sql_code = compile_sql(
        &transformation.sql_code,
        transformation.template.as_ref(),
        None,
        templates,
        config_props,
        &entity_name,
    )?;
    transformation.template = None;
    transformation.dependent_tables = get_dependent_tables(&transformation.sql_code);
    compile_operation_usages(&mut transformation.pre_runs, config_props, &entity_name)?;
    compile_operation_usages(&mut transformation.post_runs, config_props, &entity_name)?;
    compile_operation_usages(&mut transformation.init_runs, config_props, &entity_name)?;
    Ok(())
}

fn compile_operation(
    operation: &mut Operation,
    templates: &HashMap<String, Template>,
    config_props: &HashMap<String, String>,
) -> Result<()> {
    let entity_name = operation.name.clone();
    operation.sql_code = compile_sql(
        &operation.sql_code,
        operation.template.as_ref(),
        None,
        templates,
        config_props,
        &entity_name,
    )?;
    operation.template = None;
    operation.dependent_tables = get_dependent_tables(&operation.sql_code);
    Ok(())
}

fn compile_test(test: &mut Test, config_props: &HashMap<String, String>) -> Result<()> {
    let entity_name = test.name.clone();
    let mut props = config_props.clone();
    merge_props(&mut props, test.default_props.as_ref(), config_props, &entity_name)?;
    test.sql_code = render_compile_checked(&test.sql_code, &props, &entity_name, &[])?;
    test.dependent_tables = get_dependent_tables(&test.sql_code);
    Ok(())
}

fn compile_template_entity(
    template: &mut Template,
    templates: &HashMap<String, Template>,
    config_props: &HashMap<String, String>,
) -> Result<()> {
    let entity_name = template.name.clone();
    let nested = template.template.clone();
    let inner_sql = template.sql_code.clone();
    let default_props = template.default_props.clone();

    template.sql_code = if let Some(usage) = nested {
        let outer = templates.get(&usage.name).ok_or_else(|| ParseError::TemplateNotFound {
            template: usage.name.clone(),
            entity: entity_name.clone(),
        })?;
        let body_sql = resolve_template_body(outer, templates, config_props, &entity_name)?;
        let mut props = config_props.clone();
        merge_props(
            &mut props,
            outer.default_props.as_ref(),
            config_props,
            &entity_name,
        )?;
        merge_props(&mut props, default_props.as_ref(), config_props, &entity_name)?;
        let render_ctx = props.clone();
        merge_props(&mut props, usage.props.as_ref(), &render_ctx, &entity_name)?;
        let deferred_inner = {
            let mut inner_props = props.clone();
            merge_props(
                &mut inner_props,
                default_props.as_ref(),
                config_props,
                &entity_name,
            )?;
            render_template_definition(&inner_sql, &inner_props, &entity_name)?
        };
        props.insert(TEMPLATE_CALLER_PROP.to_string(), deferred_inner);
        render_compile_checked(&body_sql, &props, &entity_name, &[])?
    } else {
        let mut props = config_props.clone();
        merge_props(&mut props, default_props.as_ref(), config_props, &entity_name)?;
        render_template_definition(&inner_sql, &props, &entity_name)?
    };
    template.template = None;
    Ok(())
}

fn compile_sql(
    inner_sql: &str,
    template_usage: Option<&TemplateUsage>,
    entity_defaults: Option<&HashMap<String, String>>,
    templates: &HashMap<String, Template>,
    config_props: &HashMap<String, String>,
    entity_name: &str,
) -> Result<String> {
    if let Some(usage) = template_usage {
        let template = templates.get(&usage.name).ok_or_else(|| ParseError::TemplateNotFound {
            template: usage.name.clone(),
            entity: entity_name.to_string(),
        })?;
        let body_sql = resolve_template_body(template, templates, config_props, entity_name)?;
        let mut props = config_props.clone();
        merge_props(
            &mut props,
            template.default_props.as_ref(),
            config_props,
            entity_name,
        )?;
        merge_props(&mut props, entity_defaults, config_props, entity_name)?;
        let rendered_inner = render_compile_checked(inner_sql, &props, entity_name, &[])?;
        // Mention props apply when wrapping with the template body, not when rendering inner SQL.
        let render_ctx = props.clone();
        merge_props(&mut props, usage.props.as_ref(), &render_ctx, entity_name)?;
        props.insert("code".to_string(), rendered_inner);
        render_compile_checked(&body_sql, &props, entity_name, &[])
    } else {
        let mut props = config_props.clone();
        merge_props(&mut props, entity_defaults, config_props, entity_name)?;
        render_compile_checked(inner_sql, &props, entity_name, &[])
    }
}

fn resolve_template_body(
    template: &Template,
    templates: &HashMap<String, Template>,
    config_props: &HashMap<String, String>,
    entity_name: &str,
) -> Result<String> {
    if let Some(nested) = &template.template {
        compile_sql(
            &template.sql_code,
            Some(nested),
            template.default_props.as_ref(),
            templates,
            config_props,
            entity_name,
        )
    } else {
        Ok(template.sql_code.clone())
    }
}

fn merge_props(
    target: &mut HashMap<String, String>,
    source: Option<&HashMap<String, String>>,
    render_ctx: &HashMap<String, String>,
    entity_name: &str,
) -> Result<()> {
    if let Some(source) = source {
        for (key, value) in source {
            target.insert(
                key.clone(),
                render_compile_checked(value, render_ctx, entity_name, &[])?,
            );
        }
    }
    Ok(())
}

fn compile_operation_usages(
    usages: &mut Option<Vec<OperationUsage>>,
    config_props: &HashMap<String, String>,
    entity_name: &str,
) -> Result<()> {
    if let Some(usages) = usages {
        for usage in usages.iter_mut() {
            if let Some(props) = usage.props.as_mut() {
                for value in props.values_mut() {
                    *value = render_compile_checked(value, config_props, entity_name, &[])?;
                }
            }
        }
    }
    Ok(())
}

/// `props__code` is filled with the caller's inner SQL when a template is included.
const TEMPLATE_CALLER_PROP: &str = "code";

/// Props left for callers to fill via `TemplateUsage` when compiling a template definition.
fn template_definition_deferred_props(
    template: &str,
    props: &HashMap<String, String>,
) -> Vec<String> {
    let mut deferred = vec![TEMPLATE_CALLER_PROP.to_string()];
    for key in crate::context::render::extract_mustache_tags(template) {
        let Some(prop) = key.strip_prefix("props__") else {
            continue;
        };
        if prop == TEMPLATE_CALLER_PROP {
            continue;
        }
        let unresolved = props
            .get(prop)
            .is_none_or(|value| value.is_empty());
        if unresolved && !deferred.iter().any(|d| d == prop) {
            deferred.push(prop.to_string());
        }
    }
    deferred
}

fn render_template_definition(
    template: &str,
    props: &HashMap<String, String>,
    entity_name: &str,
) -> Result<String> {
    let deferred = template_definition_deferred_props(template, props);
    let deferred_refs: Vec<&str> = deferred.iter().map(String::as_str).collect();
    render_compile_checked(template, props, entity_name, &deferred_refs)
}

fn render_compile_checked(
    template: &str,
    props: &HashMap<String, String>,
    entity_name: &str,
    deferred: &[&str],
) -> Result<String> {
    for key in crate::context::render::extract_mustache_tags(template) {
        if let Some(prop) = key.strip_prefix("props__") {
            if deferred.contains(&prop) {
                continue;
            }
            if !props.contains_key(prop) {
                return Err(ParseError::PropNotFound {
                    prop: prop.to_string(),
                    entity: entity_name.to_string(),
                });
            }
        }
    }
    Ok(render_compile_deferred(template, props, deferred))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_spec_dir;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../specs/data-specs")
    }

    #[test]
    fn compile_preserves_caller_props_when_template_nests_template() {
        let mut entities = vec![
            (
                PathBuf::from("dedup.md"),
                Entity::Template(Template {
                    name: "dedup".into(),
                    description: None,
                    sql_code: "WITH q AS (\n    {{props__code}}\n)\nSELECT DISTINCT ON ({{props__partition_by}}) *\nFROM q\nORDER BY {{props__partition_by}}, {{props__order_by}}".into(),
                    dependent_tables: vec![],
                    used_variables: None,
                    default_props: Some(HashMap::from([
                        ("partition_by".into(), "".into()),
                        ("order_by".into(), "1".into()),
                    ])),
                    template: None,
                }),
            ),
            (
                PathBuf::from("wrapper.md"),
                Entity::Template(Template {
                    name: "wrapper".into(),
                    description: None,
                    sql_code: "SELECT {{props__start_block}} AS id\nUNION ALL\n{{props__code}}".into(),
                    dependent_tables: vec![],
                    used_variables: None,
                    default_props: None,
                    template: Some(TemplateUsage {
                        name: "dedup".into(),
                        props: Some(HashMap::from([
                            ("partition_by".into(), "id".into()),
                            ("order_by".into(), "1".into()),
                        ])),
                    }),
                }),
            ),
            (
                PathBuf::from("model.md"),
                Entity::Transformation(Transformation {
                    name: "model__default_transformation".into(),
                    sql_code: "SELECT 99 AS id".into(),
                    model: "model".into(),
                    dependent_tables: vec![],
                    used_variables: None,
                    template: Some(TemplateUsage {
                        name: "wrapper".into(),
                        props: Some(HashMap::from([("start_block".into(), "123".into())])),
                    }),
                    columns: None,
                    tests: None,
                    pre_runs: None,
                    post_runs: None,
                    init_runs: None,
                }),
            ),
        ];

        compile_entities(&mut entities, &Config::default()).unwrap();

        let Entity::Template(wrapper) = &entities[1].1 else {
            panic!("expected wrapper template");
        };
        assert!(wrapper.sql_code.contains("{{props__start_block}}"));
        assert!(wrapper.sql_code.contains("{{props__code}}"));
        assert!(wrapper.sql_code.contains("DISTINCT ON (id)"));

        let Entity::Transformation(t) = &entities[2].1 else {
            panic!("expected transformation");
        };
        assert_eq!(
            t.sql_code,
            "WITH q AS (\n    SELECT 123 AS id\nUNION ALL\nSELECT 99 AS id\n)\nSELECT DISTINCT ON (id) *\nFROM q\nORDER BY id, 1"
        );
    }

    #[test]
    fn compile_leaves_mention_props_in_template_definition() {
        let mut entities = vec![(
            PathBuf::from("wrapper.md"),
            Entity::Template(Template {
                name: "block_filter".into(),
                description: None,
                sql_code: "SELECT * FROM src WHERE block_number BETWEEN {{props__start_block}} AND {{props__end_block}}".into(),
                dependent_tables: vec![],
                used_variables: None,
                default_props: None,
                template: None,
            }),
        )];

        compile_entities(&mut entities, &Config::default()).unwrap();

        let Entity::Template(t) = &entities[0].1 else {
            panic!("expected template");
        };
        assert!(t.sql_code.contains("{{props__start_block}}"));
        assert!(t.sql_code.contains("{{props__end_block}}"));
    }

    #[test]
    fn compile_leaves_props_code_in_template_definition() {
        let mut entities = parse_spec_dir(fixture_dir()).unwrap();
        let config = entities
            .iter()
            .find_map(|(_, e)| match e {
                Entity::Config(c) => Some(c.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let mut config = config;
        config.props.insert("vata".into(), "42".into());

        compile_entities(&mut entities, &config).unwrap();

        let template = entities.iter().find_map(|(_, e)| match e {
            Entity::Template(t) if t.name == "dummy_template" => Some(t.clone()),
            _ => None,
        });
        assert!(template.is_some());
        assert!(template.unwrap().sql_code.contains("{{props__code}}"));
    }

    #[test]
    fn compile_fills_dependent_tables_after_render() {
        let mut entities = parse_spec_dir(fixture_dir()).unwrap();
        let config = entities
            .iter()
            .find_map(|(_, e)| match e {
                Entity::Config(c) => Some(c.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let mut config = config;
        config.props.insert("vata".into(), "42".into());

        compile_entities(&mut entities, &config).unwrap();

        let transformation = entities.iter().find_map(|(_, e)| match e {
            Entity::Transformation(t) if t.name == "dummy_model__default_transformation" => {
                Some(t.clone())
            }
            _ => None,
        });
        let transformation = transformation.expect("embedded model transformation");
        assert_eq!(
            transformation.dependent_tables,
            vec!["{{dummy_model}}".to_string()]
        );

        let mut entities = vec![(
            PathBuf::from("inline.md"),
            Entity::Transformation(Transformation {
                name: "t_with_tables".into(),
                sql_code: "SELECT a.id FROM alpha a JOIN beta b ON a.id = b.a_id".into(),
                model: "m".into(),
                dependent_tables: vec![],
                used_variables: None,
                template: None,
                columns: None,
                tests: None,
                pre_runs: None,
                post_runs: None,
                init_runs: None,
            }),
        )];
        compile_entities(&mut entities, &Config::default()).unwrap();
        let Entity::Transformation(t) = &entities[0].1 else {
            panic!("expected transformation");
        };
        assert_eq!(
            t.dependent_tables,
            vec!["alpha".to_string(), "beta".to_string()]
        );
    }

    #[test]
    fn compile_applies_template_mention_props_to_wrapper_only() {
        let mut entities = vec![(
            PathBuf::from("inner.md"),
            Entity::Transformation(Transformation {
                name: "t".into(),
                sql_code: "SELECT {{props__only_in_inner}}".into(),
                model: "m".into(),
                dependent_tables: vec![],
                used_variables: None,
                template: Some(TemplateUsage {
                    name: "wrapper".into(),
                    props: Some(HashMap::from([(
                        "only_in_mention".into(),
                        "mention_value".into(),
                    )])),
                }),
                columns: None,
                tests: None,
                pre_runs: None,
                post_runs: None,
                init_runs: None,
            }),
        ), (
            PathBuf::from("wrapper.md"),
            Entity::Template(Template {
                name: "wrapper".into(),
                description: None,
                sql_code: "WRAPPER {{props__only_in_mention}} CODE {{props__code}}".into(),
                dependent_tables: vec![],
                used_variables: None,
                default_props: Some(HashMap::from([("only_in_mention".into(), "".into())])),
                template: None,
            }),
        )];

        let config = Config {
            props: HashMap::from([("only_in_inner".into(), "inner_value".into())]),
        };

        compile_entities(&mut entities, &config).unwrap();

        let Entity::Transformation(t) = &entities[0].1 else {
            panic!("expected transformation");
        };
        assert_eq!(
            t.sql_code,
            "WRAPPER mention_value CODE SELECT inner_value"
        );
    }

    #[test]
    fn compile_applies_template_mention_props() {
        let mut entities = parse_spec_dir(fixture_dir()).unwrap();
        let mut config = entities
            .iter()
            .find_map(|(_, e)| match e {
                Entity::Config(c) => Some(c.clone()),
                _ => None,
            })
            .unwrap_or_default();
        config.props.insert("vata".into(), "42".into());

        // Override table_name via template mention props.
        for (_, entity) in entities.iter_mut() {
            if let Entity::Transformation(t) = entity {
                if t.name == "dummy_model_v1" {
                    if let Some(template) = t.template.as_mut() {
                        template.props = Some(HashMap::from([(
                            "table_name".into(),
                            "my_custom_table".into(),
                        )]));
                    }
                }
            }
        }

        compile_entities(&mut entities, &config).unwrap();

        let sql = entities
            .iter()
            .find_map(|(_, e)| match e {
                Entity::Transformation(t) if t.name == "dummy_model_v1" => Some(t.sql_code.clone()),
                _ => None,
            })
            .expect("dummy_model_v1");
        assert!(
            sql.contains("my_custom_table"),
            "expected mention props in compiled SQL, got: {sql}"
        );
    }

    #[test]
    fn compile_renders_props_in_sql() {
        let mut entities = parse_spec_dir(fixture_dir()).unwrap();
        let config = entities
            .iter()
            .find_map(|(_, e)| match e {
                Entity::Config(c) => Some(c.clone()),
                _ => None,
            })
            .unwrap_or_default();

        // vata is referenced in specs but may be absent from config; add for this test.
        let mut config = config;
        config.props.insert("vata".into(), "42".into());

        compile_entities(&mut entities, &config).unwrap();

        let transformation = entities.iter().find_map(|(_, e)| match e {
            Entity::Transformation(t) if t.name == "dummy_model_v1" => Some(t.clone()),
            _ => None,
        });
        assert!(transformation.is_some());
        let sql = &transformation.unwrap().sql_code;
        assert!(sql.contains("42"));
        assert!(!sql.contains("props__vata"));
        assert!(sql.contains("{{session_id}}"));
        assert!(sql.contains("{{dummy_model}}"));
    }
}
