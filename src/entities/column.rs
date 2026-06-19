use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub description: Option<String>,
    pub data_type: Option<String>,
    pub labels: Option<Vec<String>>,
    pub tests: Option<Vec<String>>,
}

impl Column {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn labels(&self) -> Option<&Vec<String>> {
        self.labels.as_ref()
    }
}
