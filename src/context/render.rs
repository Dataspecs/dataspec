use std::collections::{HashMap, HashSet};

use mustache::{compile_str, MapBuilder, Template};

use super::Ctx;

/// Compile-time rendering: substitute `{{props__<key>}}` only.
pub fn render_compile(template: &str, props: &HashMap<String, String>) -> String {
    render_compile_deferred(template, props, &[])
}

/// Like [`render_compile`], but leaves `{{props__<name>}}` untouched for deferred names.
pub fn render_compile_deferred(
    template: &str,
    props: &HashMap<String, String>,
    deferred: &[&str],
) -> String {
    render_selective(template, |key| {
        if let Some(name) = key.strip_prefix("props__") {
            if deferred.contains(&name) {
                return None;
            }
            props
                .get(name)
                .cloned()
                .map(Some)
                .unwrap_or_else(|| panic!("Prop props__{name} not found in context"))
        } else {
            None
        }
    })
}

/// Runtime rendering for executed SQL: `{{var__*}}`, `{{vars__*}}`, `{{session_id}}`, model refs.
pub fn render_runtime(template: &str, ctx: &Ctx<'_>) -> String {
    render_selective(template, |key| ctx.resolve_runtime(key))
}

/// Backwards-compatible alias for [`render_runtime`].
pub fn render(template: &str, ctx: &Ctx<'_>) -> String {
    render_runtime(template, ctx)
}

fn render_selective<F>(template: &str, resolve: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let compiled = compile_str(template).unwrap_or_else(|e| {
        panic!("Failed to compile mustache template: {e}");
    });
    let data = selective_mustache_data(&compiled, template, resolve);
    compiled
        .render_data_to_string(&data)
        .unwrap_or_else(|e| panic!("Failed to render mustache template: {e}"))
}

fn selective_mustache_data<F>(
    _template: &Template,
    source: &str,
    resolve: F,
) -> mustache::Data
where
    F: Fn(&str) -> Option<String>,
{
    let keys = extract_mustache_tags(source);
    let mut builder = MapBuilder::new();

    for key in keys {
        let value = resolve(&key).unwrap_or_else(|| format!("{{{{{key}}}}}"));
        tracing::debug!("Variable: {key} = {value:?}");
        builder = builder.insert_str(key, value);
    }

    builder.build()
}

impl Ctx<'_> {
    fn resolve_runtime(&self, key: &str) -> Option<String> {
        match key {
            "session_id" => Some(self.session_id.clone()),
            k if strip_var_prefix(k).is_some() => {
                Some(self.resolve_var(strip_var_prefix(k).unwrap()))
            }
            k if k.starts_with("props__") => None,
            _ => Some(self.resolve_model_or_mapping(key)),
        }
    }

    fn resolve_var(&self, name: &str) -> String {
        if self
            .vars
            .as_ref()
            .is_some_and(|vars| vars.contains_key(name))
        {
            return self.vars.as_ref().unwrap()[name].clone();
        }
        if self
            .env_vars
            .as_ref()
            .is_some_and(|env| env.contains_key(name))
        {
            return self.env_vars.as_ref().unwrap()[name].clone();
        }
        panic!("Variable vars__{name} not found in context");
    }

    fn resolve_model_or_mapping(&self, key: &str) -> String {
        if self
            .table_mappings
            .as_ref()
            .is_some_and(|m| m.contains_key(key))
        {
            return self.table_mappings.as_ref().unwrap()[key].clone();
        }
        if self
            .data_catalog
            .is_some_and(|c| c.models_by_name.contains_key(key))
        {
            let catalog = self.data_catalog.unwrap();
            let model = catalog.models_by_name.get(key).unwrap();
            return model
                .table_id
                .clone()
                .unwrap_or_else(|| model.name().to_string());
        }
        panic!("Model {key} not found in context");
    }
}

fn strip_var_prefix(key: &str) -> Option<&str> {
    key.strip_prefix("vars__")
        .or_else(|| key.strip_prefix("var__"))
}

pub(crate) fn extract_mustache_tags(source: &str) -> HashSet<String> {
    let mut keys = HashSet::new();
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("}}") else {
            break;
        };
        let tag = rest[..end].trim();
        if !tag.is_empty()
            && !tag.starts_with('#')
            && !tag.starts_with('^')
            && !tag.starts_with('/')
        {
            keys.insert(tag.to_string());
        }
        rest = &rest[end + 2..];
    }
    keys
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::entities::{Config, DataCatalog, Model};

    fn catalog_with_model() -> DataCatalog {
        let mut catalog = DataCatalog::new();
        catalog.register_config(Config {
            props: HashMap::from([("vata".into(), "42".into())]),
        });
        let model = Box::leak(Box::new(Model {
            name: "dummy_model".into(),
            description: None,
            tags: None,
            table_id: Some("dataset.dummy_model".into()),
            managed: false,
            disabled: false,
            meta: None,
            default_transformation: None,
        }));
        catalog.register_model(model);
        catalog
    }

    #[test]
    fn render_runtime_substitutes_vars_session_id_and_model() {
        let catalog = catalog_with_model();
        let mut ctx = Ctx::new();
        ctx.set_vars(HashMap::from([("report_year".into(), "2024".into())]));
        ctx.set_env_vars(HashMap::new());
        ctx.set_data_catalog(&catalog);

        let sql = "SELECT * FROM {{dummy_model}} WHERE year = {{vars__report_year}} AND sid = {{session_id}}";
        let rendered = render_runtime(sql, &ctx);

        assert!(rendered.contains("dataset.dummy_model"));
        assert!(rendered.contains("2024"));
        assert!(!rendered.contains("{{"));
    }

    #[test]
    fn render_compile_substitutes_props_only() {
        let props = HashMap::from([("vata".into(), "42".into())]);
        let sql = "SELECT * FROM {{dummy_model}} WHERE f = {{props__vata}}";
        let rendered = render_compile(&sql, &props);

        assert_eq!(rendered, "SELECT * FROM {{dummy_model}} WHERE f = 42");
    }

    #[test]
    fn render_runtime_leaves_props_untouched() {
        let catalog = catalog_with_model();
        let mut ctx = Ctx::new();
        ctx.set_data_catalog(&catalog);

        let sql = "SELECT * FROM {{dummy_model}} WHERE f = {{props__vata}}";
        let rendered = render_runtime(sql, &ctx);

        assert_eq!(rendered, "SELECT * FROM dataset.dummy_model WHERE f = {{props__vata}}");
    }

    #[test]
    fn render_compile_leaves_runtime_vars_untouched() {
        let props = HashMap::from([("vata".into(), "42".into())]);
        let sql = "SELECT *, {{session_id}} FROM {{dummy_model}} WHERE f = {{props__vata}}";
        let rendered = render_compile(&sql, &props);

        assert_eq!(
            rendered,
            "SELECT *, {{session_id}} FROM {{dummy_model}} WHERE f = 42"
        );
    }

    #[test]
    fn render_runtime_falls_back_to_env_for_vars() {
        let catalog = catalog_with_model();
        let mut ctx = Ctx::new();
        ctx.set_vars(HashMap::new());
        ctx.set_env_vars(HashMap::from([("report_year".into(), "2023".into())]));
        ctx.set_data_catalog(&catalog);

        let sql = "SELECT {{vars__report_year}}";
        let rendered = render_runtime(sql, &ctx);

        assert_eq!(rendered, "SELECT 2023");
    }

    #[test]
    fn render_runtime_supports_var_prefix_alias() {
        let catalog = catalog_with_model();
        let mut ctx = Ctx::new();
        ctx.set_vars(HashMap::from([("report_year".into(), "2024".into())]));
        ctx.set_env_vars(HashMap::new());
        ctx.set_data_catalog(&catalog);

        let sql = "SELECT {{var__report_year}}";
        let rendered = render_runtime(sql, &ctx);

        assert_eq!(rendered, "SELECT 2024");
    }

    #[test]
    #[should_panic(expected = "Variable vars__missing not found")]
    fn render_runtime_panics_on_missing_var() {
        let catalog = catalog_with_model();
        let mut ctx = Ctx::new();
        ctx.set_data_catalog(&catalog);
        render_runtime("SELECT {{vars__missing}}", &ctx);
    }

    #[test]
    #[should_panic(expected = "Prop props__missing not found")]
    fn render_compile_panics_on_missing_prop() {
        render_compile("SELECT {{props__missing}}", &HashMap::new());
    }
}
