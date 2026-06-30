use std::collections::HashMap;

use crate::context::render_compile_deferred;
use crate::entities::{
    Config, Entity, Operation, OperationUsage, Template, TemplateUsage, Test, Transformation,
};
use crate::error::{ParseError, Result};

/// Compile-time rendering: inline templates and substitute `{{props__*}}`.
pub fn compile_entities(entities: &mut [(std::path::PathBuf, Entity)], config: &Config) -> Result<()> {
    let templates = index_templates(entities);

    for (_, entity) in entities.iter_mut() {
        match entity {
            Entity::Transformation(t) => compile_transformation(t, &templates, &config.props)?,
            Entity::Operation(o) => compile_operation(o, &templates, &config.props)?,
            Entity::Test(t) => compile_test(t, &config.props)?,
            Entity::Template(t) => compile_template_entity(t, &templates, &config.props)?,
            Entity::Config(_) | Entity::Model(_) => {}
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
    Ok(())
}

fn compile_test(test: &mut Test, config_props: &HashMap<String, String>) -> Result<()> {
    let entity_name = test.name.clone();
    let mut props = config_props.clone();
    merge_props(&mut props, test.default_props.as_ref(), config_props, &entity_name)?;
    test.sql_code = render_compile_checked(&test.sql_code, &props, &entity_name, &[])?;
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
        compile_sql(
            &inner_sql,
            Some(&usage),
            default_props.as_ref(),
            templates,
            config_props,
            &entity_name,
        )?
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
        merge_props(&mut props, usage.props.as_ref(), config_props, entity_name)?;
        merge_props(&mut props, entity_defaults, config_props, entity_name)?;
        let rendered_inner = render_compile_checked(inner_sql, &props, entity_name, &[])?;
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
const TEMPLATE_CALLER_PROPS: &[&str] = &["code"];

fn render_template_definition(
    template: &str,
    props: &HashMap<String, String>,
    entity_name: &str,
) -> Result<String> {
    render_compile_checked(template, props, entity_name, TEMPLATE_CALLER_PROPS)
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
