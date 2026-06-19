use serde::{Deserialize, Serialize};

use crate::entities::execution_plan::{ExecutionStep, ExecutionStepType};
use crate::entities::template_usage::TemplateUsage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub description: Option<String>,
    pub sql_code: String,
    pub dependent_tables: Vec<String>,
    pub used_variables: Option<Vec<String>>,
    pub template: Option<TemplateUsage>,
}

impl Template {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl ExecutionStep for Template {
    fn name(&self) -> &str {
        &self.name
    }

    fn sql(&self) -> &str {
        &self.sql_code
    }

    fn step_type(&self) -> ExecutionStepType {
        ExecutionStepType::Template
    }
}
