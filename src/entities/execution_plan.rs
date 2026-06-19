use serde::Serialize;
use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone, Serialize)]
pub enum ExecutionStepType {
    Model,
    Operation,
    Transformation,
    Template,
    Test,
}

impl Display for ExecutionStepType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionStepJson {
    pub name: String,
    pub sql: String,
    pub step_type: ExecutionStepType,
}

pub trait ExecutionStep {
    fn name(&self) -> &str;
    fn sql(&self) -> &str;
    fn step_type(&self) -> ExecutionStepType;

    fn to_json(&self) -> ExecutionStepJson {
        ExecutionStepJson {
            name: self.name().to_string(),
            sql: self.sql().to_string(),
            step_type: self.step_type(),
        }
    }
}

pub struct ExecutionPlan {
    steps: Vec<Vec<Box<dyn ExecutionStep>>>,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    pub fn add_steps(&mut self, steps: Vec<Box<dyn ExecutionStep>>) {
        self.steps.push(steps);
    }

    pub fn get_steps(&self) -> &Vec<Vec<Box<dyn ExecutionStep>>> {
        &self.steps
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}
