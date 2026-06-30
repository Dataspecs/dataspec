use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::entities::execution_plan::{ExecutionStep, ExecutionStepType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Test {
    pub name: String,
    pub description: Option<String>,
    pub sql_code: String,
    pub dependent_tables: Vec<String>,
    pub used_variables: Option<Vec<String>>,
    pub default_props: Option<HashMap<String, String>>,
}

impl Test {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl ExecutionStep for Test {
    fn name(&self) -> &str {
        &self.name
    }

    fn sql(&self) -> &str {
        &self.sql_code
    }

    fn step_type(&self) -> ExecutionStepType {
        ExecutionStepType::Test
    }
}
