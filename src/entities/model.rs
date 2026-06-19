use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::entities::execution_plan::{ExecutionStep, ExecutionStepType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub table_id: Option<String>,
    pub managed: bool,
    pub disabled: bool,
    pub meta: Option<HashMap<String, String>>,
    pub default_transformation: Option<String>,
}

impl Model {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl ExecutionStep for Model {
    fn name(&self) -> &str {
        &self.name
    }

    fn sql(&self) -> &str {
        ""
    }

    fn step_type(&self) -> ExecutionStepType {
        ExecutionStepType::Model
    }
}
