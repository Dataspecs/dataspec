use std::collections::HashMap;

use subst::VariableMap;
use uuid::Uuid;

use crate::entities::DataCatalog;

#[derive(Debug)]
pub struct Ctx<'a> {
    pub props: Option<HashMap<String, String>>,
    pub vars: Option<HashMap<String, String>>,
    pub env_vars: Option<HashMap<String, String>>,
    pub session_id: String,
    pub table_mappings: Option<HashMap<String, String>>,
    pub data_catalog: Option<&'a DataCatalog>,
}

impl<'a> Ctx<'a> {
    pub fn new() -> Ctx<'a> {
        Ctx {
            props: Some(HashMap::new()),
            vars: Some(HashMap::new()),
            env_vars: Some(HashMap::new()),
            session_id: Uuid::new_v4().simple().to_string(),
            table_mappings: None,
            data_catalog: None,
        }
    }

    pub fn set_vars(&mut self, vars: HashMap<String, String>) {
        self.vars = Some(vars);
    }

    pub fn set_env_vars(&mut self, env_vars: HashMap<String, String>) {
        self.env_vars = Some(env_vars);
    }

    pub fn set_table_mappings(&mut self, table_mappings: HashMap<String, String>) {
        self.table_mappings = Some(table_mappings);
    }

    pub fn set_data_catalog(&mut self, data_catalog: &'a DataCatalog) {
        self.data_catalog = Some(data_catalog);
        self.props = Some(data_catalog.config.props.clone());
    }
}

impl Default for Ctx<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> VariableMap<'a> for Ctx<'a> {
    type Value = String;

    fn get(&self, key: &str) -> Option<String> {
        let val = match key {
            "session_id" => Some(self.session_id.to_string()),
            k if k.starts_with("vars__") => {
                let var_name = k.strip_prefix("vars__").unwrap();
                if self.vars.as_ref()?.contains_key(var_name) {
                    self.vars.as_ref()?.get(var_name).cloned()
                } else if self.env_vars.as_ref()?.contains_key(var_name) {
                    self.env_vars.as_ref()?.get(var_name).cloned()
                } else {
                    panic!("Variable ${key} not found in context")
                }
            }
            _ => {
                if self
                    .table_mappings
                    .as_ref()
                    .is_some_and(|m| m.contains_key(key))
                {
                    self.table_mappings.as_ref()?.get(key).cloned()
                } else if self
                    .data_catalog
                    .is_some_and(|c| c.models_by_name.contains_key(key))
                {
                    let catalog = self.data_catalog.unwrap();
                    catalog.models_by_name.get(key).map(|m| {
                        m.table_id
                            .clone()
                            .unwrap_or_else(|| m.name().to_string())
                    })
                } else {
                    panic!("Model ${key} not found in context")
                }
            }
        };
        tracing::debug!("Variable: {key} = {val:?}");
        val
    }
}
