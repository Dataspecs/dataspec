use std::collections::{HashMap, HashSet};

use crate::entities::config::Config;
use crate::entities::execution_plan::ExecutionStep;
use crate::entities::model::Model;
use crate::entities::operation::Operation;
use crate::entities::template::Template;
use crate::entities::test::Test;
use crate::entities::transformation::Transformation;

#[derive(Debug)]
pub struct DataCatalog {
    pub models_up_deps: HashMap<String, Vec<&'static Model>>,
    pub models_down_deps: HashMap<String, Vec<&'static Model>>,
    pub models_by_name: HashMap<String, &'static Model>,
    pub models_by_tag: HashMap<String, Vec<&'static Model>>,

    pub operations_down_deps: HashMap<String, Vec<&'static Operation>>,
    pub operations_by_name: HashMap<String, &'static Operation>,
    pub operations_by_tag: HashMap<String, Vec<&'static Operation>>,

    pub transformations_by_name: HashMap<String, &'static Transformation>,

    pub templates_by_name: HashMap<String, &'static Template>,

    pub tests_by_name: HashMap<String, &'static Test>,

    pub config: Config,
}

impl DataCatalog {
    pub fn new() -> DataCatalog {
        DataCatalog {
            models_up_deps: HashMap::new(),
            models_down_deps: HashMap::new(),
            models_by_name: HashMap::new(),
            models_by_tag: HashMap::new(),

            operations_down_deps: HashMap::new(),
            operations_by_name: HashMap::new(),
            operations_by_tag: HashMap::new(),

            transformations_by_name: HashMap::new(),
            templates_by_name: HashMap::new(),
            tests_by_name: HashMap::new(),
            config: Config::new(),
        }
    }

    pub fn register_model(&mut self, model: &'static Model) {
        self.models_by_name.insert(model.name.clone(), model);
    }

    pub fn register_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn register_models(&mut self, models: Vec<&'static Model>) {
        for model in models {
            self.register_model(model);
            if let Some(tags) = &model.tags {
                for tag in tags.iter() {
                    self.models_by_tag
                        .entry(tag.to_string())
                        .or_insert(Vec::new())
                        .push(model);
                }
            }
        }
    }

    pub fn register_operation(&mut self, operation: &'static Operation) {
        self.operations_by_name
            .insert(operation.name.clone(), operation);
    }

    pub fn register_operations(&mut self, operations: Vec<&'static Operation>) {
        for operation in operations {
            self.register_operation(operation);
            if let Some(tags) = &operation.tags {
                for tag in tags.iter() {
                    self.operations_by_tag
                        .entry(tag.to_string())
                        .or_insert(Vec::new())
                        .push(operation);
                }
            }
        }
    }

    pub fn register_transformation(&mut self, transformation: &'static Transformation) {
        self.transformations_by_name
            .insert(transformation.name.clone(), transformation);
    }

    pub fn register_transformations(&mut self, transformations: Vec<&'static Transformation>) {
        for transformation in transformations {
            self.register_transformation(transformation);
        }
    }

    pub fn register_template(&mut self, template: &'static Template) {
        self.templates_by_name
            .insert(template.name.clone(), template);
    }

    pub fn register_templates(&mut self, templates: Vec<&'static Template>) {
        for template in templates {
            self.register_template(template);
        }
    }

    pub fn register_test(&mut self, test: &'static Test) {
        self.tests_by_name.insert(test.name.clone(), test);
    }

    pub fn register_tests(&mut self, tests: Vec<&'static Test>) {
        for test in tests {
            self.register_test(test);
        }
    }

    pub fn get_execution_pipeline_by_model_name(
        &self,
        name: &str,
        transformation_name: Option<&str>,
    ) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let model = self.models_by_name.get(name)?;

        let transformation: &Transformation = if let Some(transformation_name) = transformation_name
        {
            self.transformations_by_name.get(transformation_name)?
        } else {
            self.transformations_by_name
                .get(model.default_transformation.as_ref()?)?
        };
        Some(vec![Box::new(transformation.clone())])
    }

    pub fn lookup_model_by_name(&self, name: &str) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let model = self.models_by_name.get(name)?;
        Some(vec![Box::new((*model).clone())])
    }

    pub fn lookup_operation_by_name(&self, name: &str) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let operation = self.operations_by_name.get(name)?;
        Some(vec![Box::new((*operation).clone())])
    }

    pub fn lookup_transformation_by_name(&self, name: &str) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let transformation = self.transformations_by_name.get(name)?;
        Some(vec![Box::new((*transformation).clone())])
    }

    pub fn lookup_template_by_name(&self, name: &str) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let template = self.templates_by_name.get(name)?;
        Some(vec![Box::new((*template).clone())])
    }

    pub fn lookup_test_by_name(&self, name: &str) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let test = self.tests_by_name.get(name)?;
        Some(vec![Box::new((*test).clone()) as Box<dyn ExecutionStep>])
    }

    fn sort_models_by_deps(&self, mut models: Vec<&'static Model>) -> Vec<&'static Model> {
        models.sort_by(|a, b| {
            let a_deps: HashSet<&str> = self
                .models_down_deps
                .get(a.name())
                .map(|deps| deps.iter().map(|dep| dep.name()).collect())
                .unwrap_or_default();
            let b_deps: HashSet<&str> = self
                .models_down_deps
                .get(b.name())
                .map(|deps| deps.iter().map(|dep| dep.name()).collect())
                .unwrap_or_default();

            if a_deps.contains(b.name()) {
                std::cmp::Ordering::Less
            } else if b_deps.contains(a.name()) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        models
    }

    pub fn lookup_models_by_tags(&self, tags: Vec<String>) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let mut models = Vec::new();
        for tag in tags {
            if let Some(found_models) = self.models_by_tag.get(&tag) {
                models.extend(found_models.iter().cloned());
            }
        }
        let dependency_sorted_models =
            self.sort_models_by_deps(models.into_iter().collect::<Vec<&'static Model>>().clone());

        let transformations = dependency_sorted_models
            .into_iter()
            .map(|model| {
                self.transformations_by_name
                    .get(model.default_transformation.as_ref().unwrap())
                    .map(|t| *t)
            })
            .collect::<Option<Vec<&'static Transformation>>>()?;
        Some(
            transformations
                .into_iter()
                .map(|transformation| Box::new((*transformation).clone()) as Box<dyn ExecutionStep>)
                .collect(),
        )
    }

    pub fn lookup_operations_by_tags(
        &self,
        tags: Vec<String>,
    ) -> Option<Vec<Box<dyn ExecutionStep>>> {
        let mut operations = Vec::new();
        for tag in tags {
            if let Some(found_operations) = self.operations_by_tag.get(&tag) {
                operations.extend(
                    found_operations
                        .iter()
                        .map(|operation| Box::new((*operation).clone()) as Box<dyn ExecutionStep>),
                );
            }
        }
        Some(operations)
    }

    pub fn all_models(&self) -> Vec<Box<dyn ExecutionStep>> {
        self.models_by_name
            .values()
            .map(|model| Box::new((*model).clone()) as Box<dyn ExecutionStep>)
            .collect()
    }

    pub fn all_operations(&self) -> Vec<Box<dyn ExecutionStep>> {
        self.operations_by_name
            .values()
            .map(|operation| Box::new((*operation).clone()) as Box<dyn ExecutionStep>)
            .collect()
    }

    pub fn all_transformations(&self) -> Vec<Box<dyn ExecutionStep>> {
        self.transformations_by_name
            .values()
            .map(|transformation| Box::new((*transformation).clone()) as Box<dyn ExecutionStep>)
            .collect()
    }

    pub fn all_templates(&self) -> Vec<Box<dyn ExecutionStep>> {
        self.templates_by_name
            .values()
            .map(|template| Box::new((*template).clone()) as Box<dyn ExecutionStep>)
            .collect()
    }

    pub fn all_tests(&self) -> Vec<Box<dyn ExecutionStep>> {
        self.tests_by_name
            .values()
            .map(|test| Box::new((*test).clone()) as Box<dyn ExecutionStep>)
            .collect()
    }
}

impl Default for DataCatalog {
    fn default() -> Self {
        Self::new()
    }
}
