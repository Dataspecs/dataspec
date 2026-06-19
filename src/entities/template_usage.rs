use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateUsage {
    pub name: String,
    pub props: Option<HashMap<String, String>>,
}
