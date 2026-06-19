use serde::{Deserialize, Serialize};

use crate::entities::execution_plan::{ExecutionStep, ExecutionStepType};
use crate::entities::template_usage::TemplateUsage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Operation {
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub sql_code: String,
    pub template: Option<TemplateUsage>,
    pub dependent_tables: Vec<String>,
    pub used_variables: Option<Vec<String>>,
}

impl Operation {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn dependent_tables(&self) -> &Vec<String> {
        &self.dependent_tables
    }
}

impl ExecutionStep for Operation {
    fn name(&self) -> &str {
        &self.name
    }

    fn sql(&self) -> &str {
        &self.sql_code
    }

    fn step_type(&self) -> ExecutionStepType {
        ExecutionStepType::Operation
    }
}
