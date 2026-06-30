use std::collections::HashMap;

use uuid::Uuid;

use crate::entities::DataCatalog;

#[derive(Debug)]
pub struct Ctx<'a> {
    pub vars: Option<HashMap<String, String>>,
    pub env_vars: Option<HashMap<String, String>>,
    pub session_id: String,
    pub table_mappings: Option<HashMap<String, String>>,
    pub data_catalog: Option<&'a DataCatalog>,
}

impl<'a> Ctx<'a> {
    pub fn config_props(&self) -> &HashMap<String, String> {
        self.data_catalog
            .map(|catalog| &catalog.config.props)
            .expect("data_catalog must be set before accessing config props")
    }
    pub fn new() -> Ctx<'a> {
        Ctx {
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
    }
}

impl Default for Ctx<'_> {
    fn default() -> Self {
        Self::new()
    }
}
