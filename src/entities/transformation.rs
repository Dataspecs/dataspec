use serde::{Deserialize, Serialize};

use crate::entities::column::Column;
use crate::entities::execution_plan::{ExecutionStep, ExecutionStepType};
use crate::entities::operation_usage::OperationUsage;
use crate::entities::template_usage::TemplateUsage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transformation {
    pub name: String,
    pub sql_code: String,
    pub model: String,
    pub dependent_tables: Vec<String>,
    pub used_variables: Option<Vec<String>>,
    pub template: Option<TemplateUsage>,
    pub columns: Option<Vec<Column>>,
    pub tests: Option<Vec<String>>,
    pub pre_runs: Option<Vec<OperationUsage>>,
    pub post_runs: Option<Vec<OperationUsage>>,
    pub init_runs: Option<Vec<OperationUsage>>,
}

impl Transformation {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sql_code(&self) -> &str {
        &self.sql_code
    }

    pub fn dependent_tables(&self) -> &Vec<String> {
        &self.dependent_tables
    }
}

impl ExecutionStep for Transformation {
    fn name(&self) -> &str {
        &self.name
    }

    fn sql(&self) -> &str {
        &self.sql_code
    }

    fn step_type(&self) -> ExecutionStepType {
        ExecutionStepType::Transformation
    }
}
