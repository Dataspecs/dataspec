pub mod column;
pub mod config;
pub mod data_catalog;
pub mod execution_plan;
pub mod model;
pub mod operation;
pub mod operation_usage;
pub mod template;
pub mod template_usage;
pub mod test;
pub mod transformation;

pub use column::*;
pub use config::*;
pub use data_catalog::*;
pub use execution_plan::*;
pub use model::*;
pub use operation::*;
pub use operation_usage::*;
pub use template::*;
pub use template_usage::*;
pub use test::*;
pub use transformation::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entity {
    Config(Config),
    Model(Model),
    Transformation(Transformation),
    Template(Template),
    Test(Test),
    Operation(Operation),
}

impl Entity {
    pub fn name(&self) -> &str {
        match self {
            Entity::Config(_) => "config",
            Entity::Model(e) => &e.name,
            Entity::Transformation(e) => &e.name,
            Entity::Template(e) => &e.name,
            Entity::Test(e) => &e.name,
            Entity::Operation(e) => &e.name,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Entity::Config(_) => "config",
            Entity::Model(_) => "model",
            Entity::Transformation(_) => "transformation",
            Entity::Template(_) => "template",
            Entity::Test(_) => "test",
            Entity::Operation(_) => "operation",
        }
    }
}
