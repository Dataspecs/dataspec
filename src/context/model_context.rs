use std::collections::HashMap;

use crate::entities::{Column, Model, Transformation};

/// Compile-time model view for hook operation templates.
#[derive(Clone, Debug)]
pub struct ModelContext {
    pub name: String,
    /// Mustache tag referencing the model table, e.g. `{{dummy_model}}`.
    pub handler: String,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
    pub managed: bool,
    pub disabled: bool,
    pub meta: Option<HashMap<String, String>>,
    pub columns: Vec<ColumnTemplateMeta>,
}

#[derive(Clone, Debug)]
pub struct ColumnTemplateMeta {
    pub name: String,
    pub description: Option<String>,
    pub data_type: Option<String>,
    pub labels: Option<Vec<String>>,
}

impl ColumnTemplateMeta {
    pub fn from_column(column: &Column) -> Self {
        Self {
            name: column.name.clone(),
            description: column.description.clone(),
            data_type: column.data_type.clone(),
            labels: column.labels.clone(),
        }
    }
}

impl ModelContext {
    pub fn from_transformation(model: &Model, transformation: &Transformation) -> Self {
        let columns = transformation
            .columns
            .as_ref()
            .map(|cols| cols.iter().map(ColumnTemplateMeta::from_column).collect())
            .unwrap_or_default();

        Self {
            name: model.name.clone(),
            handler: format!("{{{{{}}}}}", model.name),
            tags: model.tags.clone(),
            description: model.description.clone(),
            managed: model.managed,
            disabled: model.disabled,
            meta: model.meta.clone(),
            columns,
        }
    }
}
