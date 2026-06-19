use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationUsage {
    pub name: String,
    pub props: Option<HashMap<String, String>>,
}
