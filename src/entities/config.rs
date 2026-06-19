use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub props: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            props: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.props.get(key).map(|v| v.as_str())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
