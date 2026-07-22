use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::entities::execution_plan::{ExecutionStep, ExecutionStepType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestUsage {
    pub name: String,
    pub props: Option<HashMap<String, String>>,
    /// Test SQL compiled at build time with props and model context.
    pub sql_code: String,
}

impl ExecutionStep for TestUsage {
    fn name(&self) -> &str {
        &self.name
    }

    fn sql(&self) -> &str {
        &self.sql_code
    }

    fn step_type(&self) -> ExecutionStepType {
        ExecutionStepType::Test
    }

    fn runtime_props(&self) -> Option<&HashMap<String, String>> {
        self.props.as_ref()
    }
}
