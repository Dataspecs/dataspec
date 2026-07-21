use std::collections::HashMap;
use std::env;

use clap::{Parser, Subcommand};

use crate::context::ctx::Ctx;
use crate::engines::{Engine, TestStatus};
use crate::entities::{
    DataCatalog, ExecutionPlan, ExecutionStep, ExecutionStepJson,
};

#[derive(Parser, Debug)]
#[command(name = "dataspec-project")]
#[command(about = "Data Specs project runtime", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Transform {
        #[arg(short, long)]
        names: Option<String>,

        #[arg(short, long)]
        tags: Option<String>,

        #[arg(
            short,
            long,
            help = "Variables to pass to the transformations (e.g. var1=value1,var2=value2)"
        )]
        vars: Option<String>,

        #[arg(
            short,
            long,
            help = "Mapping between model name and table id (e.g. model1=table1,model2=table2)"
        )]
        mappings: Option<String>,

        #[arg(short, long, help = "Enable debug logging", default_value = "false")]
        debug: bool,

        #[arg(short, long, help = "Enable JSON result output", default_value = "false")]
        json: bool,

        #[arg(
            long,
            help = "Run init hooks before pre/transformation/post hooks"
        )]
        init: bool,
    },
    Apply {
        #[arg(short, long, help = "Operation names to execute (comma-separated)")]
        names: Option<String>,

        #[arg(short, long, help = "Operation tags to execute (comma-separated)")]
        tags: Option<String>,

        #[arg(
            short,
            long,
            help = "Variables to pass to operations (e.g. var1=value1,var2=value2)"
        )]
        vars: Option<String>,

        #[arg(
            short,
            long,
            help = "Mapping between model name and table id (e.g. model1=table1,model2=table2)"
        )]
        mappings: Option<String>,

        #[arg(short, long, help = "Enable debug logging", default_value = "false")]
        debug: bool,

        #[arg(short, long, help = "Enable JSON result output", default_value = "false")]
        json: bool,
    },
    Test {
        #[arg(short, long, help = "Model names to test (comma-separated, optional model::transformation)")]
        names: Option<String>,

        #[arg(short, long, help = "Model tags whose tests to run (comma-separated)")]
        tags: Option<String>,

        #[arg(
            short,
            long,
            help = "Variables to pass to tests (e.g. var1=value1,var2=value2)"
        )]
        vars: Option<String>,

        #[arg(
            short,
            long,
            help = "Mapping between model name and table id (e.g. model1=table1,model2=table2)"
        )]
        mappings: Option<String>,

        #[arg(short, long, help = "Enable debug logging", default_value = "false")]
        debug: bool,

        #[arg(short, long, help = "Enable JSON result output", default_value = "false")]
        json: bool,
    },
    List {
        #[arg(short, long, help = "List by name")]
        names: Option<String>,

        #[arg(long, help = "List by tags")]
        tags: Option<String>,

        #[arg(short, long, help = "Enable JSON result output", default_value = "false")]
        json: bool,

        #[arg(long, help = "List models", default_value = "false")]
        models: bool,

        #[arg(long, help = "List operations", default_value = "false")]
        operations: bool,

        #[arg(long, help = "List transformations", default_value = "false")]
        transformations: bool,

        #[arg(long, help = "List templates", default_value = "false")]
        templates: bool,

        #[arg(long, help = "List tests", default_value = "false")]
        tests: bool,
    },
}

#[derive(Debug, Clone)]
struct RuntimeArgs {
    names: Option<String>,
    tags: Option<String>,
    vars: Option<String>,
    mappings: Option<String>,
    debug: bool,
    json: bool,
    init: bool,
}

fn parse_key_value_pairs(input: &str) -> HashMap<String, String> {
    input
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => Some((key.to_string(), value.to_string())),
                _ => None,
            }
        })
        .collect()
}

fn parse_cli_arg_list(input: &str) -> Vec<String> {
    input.split(',').map(|s| s.to_string()).collect()
}

fn parse_model_name(name: &str) -> (&str, Option<&str>) {
    if let Some((model_name, transformation_name)) = name.split_once("::") {
        (model_name, Some(transformation_name))
    } else {
        (name, None)
    }
}

fn log_level(json: bool, debug: bool) -> tracing::level_filters::LevelFilter {
    if json {
        // Suppress info/debug on stdout, but keep errors visible.
        tracing::level_filters::LevelFilter::ERROR
    } else if debug {
        tracing::level_filters::LevelFilter::DEBUG
    } else {
        tracing::level_filters::LevelFilter::INFO
    }
}

fn exit_with_error(message: impl std::fmt::Display) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

fn init_tracing(level: tracing::level_filters::LevelFilter) {
    tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(false)
        .with_target(false)
        .with_thread_names(false)
        .with_max_level(level)
        .init();
}

fn setup_ctx<'a>(catalog: &'a DataCatalog, vars: Option<&str>, mappings: Option<&str>) -> Ctx<'a> {
    let mut ctx = Ctx::new();
    ctx.set_vars(vars.map(parse_key_value_pairs).unwrap_or_default());
    ctx.set_env_vars(env::vars().collect());
    ctx.set_table_mappings(mappings.map(parse_key_value_pairs).unwrap_or_default());
    ctx.set_data_catalog(catalog);
    ctx
}

fn build_transform_plan(catalog: &DataCatalog, args: &RuntimeArgs) -> ExecutionPlan {
    let mut execution_plan = ExecutionPlan::new();

    if let Some(tags) = &args.tags {
        if let Some(steps) = catalog.lookup_models_by_tags(parse_cli_arg_list(tags), args.init) {
            execution_plan.add_steps(steps);
        }
    }
    if let Some(names) = &args.names {
        let plan: Vec<Vec<Box<dyn ExecutionStep>>> = parse_cli_arg_list(names)
            .iter()
            .map(|name| {
                let (model_name, transformation_name) = parse_model_name(name);
                catalog
                    .get_execution_pipeline_by_model_name(model_name, transformation_name, args.init)
                    .ok_or_else(|| format!("Can't find model {name}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to find all models");

        for steps in plan {
            execution_plan.add_steps(steps);
        }
    }

    execution_plan
}

fn build_apply_plan(catalog: &DataCatalog, args: &RuntimeArgs) -> ExecutionPlan {
    let mut execution_plan = ExecutionPlan::new();

    if let Some(tags) = &args.tags {
        if let Some(steps) = catalog.lookup_operations_by_tags(parse_cli_arg_list(tags)) {
            execution_plan.add_steps(steps);
        }
    }
    if let Some(names) = &args.names {
        let plan: Vec<Vec<Box<dyn ExecutionStep>>> = parse_cli_arg_list(names)
            .iter()
            .map(|name| {
                catalog
                    .lookup_operation_by_name(name)
                    .ok_or_else(|| format!("Can't find operation {name}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to find all operations");

        for steps in plan {
            execution_plan.add_steps(steps);
        }
    }

    execution_plan
}

fn build_test_plan(catalog: &DataCatalog, args: &RuntimeArgs) -> ExecutionPlan {
    let mut execution_plan = ExecutionPlan::new();

    if let Some(tags) = &args.tags {
        if let Some(steps) = catalog.lookup_tests_by_model_tags(parse_cli_arg_list(tags)) {
            execution_plan.add_steps(steps);
        }
    }
    if let Some(names) = &args.names {
        let plan: Vec<Vec<Box<dyn ExecutionStep>>> = parse_cli_arg_list(names)
            .iter()
            .map(|name| {
                let (model_name, transformation_name) = parse_model_name(name);
                catalog
                    .get_tests_for_model(model_name, transformation_name)
                    .ok_or_else(|| format!("Can't find model {name}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to find all models");

        for steps in plan {
            execution_plan.add_steps(steps);
        }
    }

    execution_plan
}

async fn execute_plan(
    execution_plan: &ExecutionPlan,
    ctx: &Ctx<'_>,
    level: tracing::level_filters::LevelFilter,
    json_output: bool,
) {
    init_tracing(level);

    let engine = match Engine::from_provider(ctx).await {
        Ok(eng) => eng,
        Err(e) => exit_with_error(format!("Failed to init engine: {e}")),
    };

    tracing::info!("Session ID: {}", ctx.session_id);
    tracing::info!("Starting execution plan");
    match engine.execute_plan(execution_plan, ctx).await {
        Ok(results) => {
            tracing::info!(
                "Total bytes processed: {}",
                results.result.total_bytes_processed.unwrap_or(0)
            );
            tracing::info!(
                "Number of DML affected rows: {}",
                results.result.num_dml_affected_rows.unwrap_or(0)
            );
            tracing::info!("Cache hit: {}", results.result.cache_hit.unwrap_or(false));
            tracing::info!("Bytes billed: {}", results.result.bytes_billed.unwrap_or(0));
            if json_output {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
            }
        }
        Err(e) => exit_with_error(format!("Execution failed: {e}")),
    }
    tracing::info!("Execution plan completed");
}

async fn execute_test_plan(
    execution_plan: &ExecutionPlan,
    ctx: &Ctx<'_>,
    level: tracing::level_filters::LevelFilter,
    json_output: bool,
) {
    init_tracing(level);

    let engine = match Engine::from_provider(ctx).await {
        Ok(eng) => eng,
        Err(e) => exit_with_error(format!("Failed to init engine: {e}")),
    };

    tracing::info!("Session ID: {}", ctx.session_id);
    tracing::info!("Starting test execution");
    match engine.execute_test_plan(execution_plan, ctx).await {
        Ok(results) => {
            let mut passed = 0usize;
            let mut failed = 0usize;
            let mut errored = 0usize;

            for step_result in &results.step_results {
                match step_result.test_status {
                    Some(TestStatus::Success) => {
                        passed += 1;
                        tracing::info!("PASS {}", step_result.step.name);
                    }
                    Some(TestStatus::Failed) => {
                        failed += 1;
                        tracing::error!(
                            "FAIL {} ({} rows)",
                            step_result.step.name,
                            step_result.result.num_rows.unwrap_or(0)
                        );
                    }
                    Some(TestStatus::Error) => {
                        errored += 1;
                        tracing::error!(
                            "ERROR {}: {}",
                            step_result.step.name,
                            step_result
                                .error
                                .as_deref()
                                .unwrap_or("unknown error")
                        );
                    }
                    None => {}
                }
            }

            tracing::info!(
                "Tests completed: {passed} passed, {failed} failed, {errored} errored"
            );
            if json_output {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
            }
            if failed > 0 || errored > 0 {
                std::process::exit(1);
            }
        }
        Err(e) => exit_with_error(format!("Test execution failed: {e}")),
    }
    tracing::info!("Test execution completed");
}

async fn run_with_runtime_args(
    catalog: &DataCatalog,
    args: RuntimeArgs,
    build_plan: fn(&DataCatalog, &RuntimeArgs) -> ExecutionPlan,
) {
    let level = log_level(args.json, args.debug);
    let ctx = setup_ctx(catalog, args.vars.as_deref(), args.mappings.as_deref());
    let execution_plan = build_plan(catalog, &args);
    execute_plan(&execution_plan, &ctx, level, args.json).await;
}

async fn run_tests(
    catalog: &DataCatalog,
    args: RuntimeArgs,
) {
    let level = log_level(args.json, args.debug);
    let ctx = setup_ctx(catalog, args.vars.as_deref(), args.mappings.as_deref());
    let execution_plan = build_test_plan(catalog, &args);
    execute_test_plan(&execution_plan, &ctx, level, args.json).await;
}

/// Runtime CLI handler for generated data-spec projects.
pub async fn spec_handler(catalog: &DataCatalog) {
    let cli = Cli::parse();

    match cli.command {
        Commands::Transform {
            names,
            tags,
            vars,
            mappings,
            debug,
            json,
            init,
        } => {
            run_with_runtime_args(
                catalog,
                RuntimeArgs {
                    names,
                    tags,
                    vars,
                    mappings,
                    debug,
                    json,
                    init,
                },
                build_transform_plan,
            )
            .await;
        }
        Commands::Apply {
            names,
            tags,
            vars,
            mappings,
            debug,
            json,
        } => {
            run_with_runtime_args(
                catalog,
                RuntimeArgs {
                    names,
                    tags,
                    vars,
                    mappings,
                    debug,
                    json,
                    init: false,
                },
                build_apply_plan,
            )
            .await;
        }
        Commands::Test {
            names,
            tags,
            vars,
            mappings,
            debug,
            json,
        } => {
            run_tests(
                catalog,
                RuntimeArgs {
                    names,
                    tags,
                    vars,
                    mappings,
                    debug,
                    json,
                    init: false,
                },
            )
            .await;
        }
        Commands::List {
            names,
            tags,
            json,
            models,
            operations,
            transformations,
            templates,
            tests,
        } => {
            let level = log_level(json, false);

            init_tracing(level);

            let mut execution_steps: Vec<Vec<Box<dyn ExecutionStep>>> = Vec::new();

            if names.is_none() && tags.is_none() {
                if models {
                    execution_steps.push(catalog.all_models());
                }
                if operations {
                    execution_steps.push(catalog.all_operations());
                }
                if transformations {
                    execution_steps.push(catalog.all_transformations());
                }
                if templates {
                    execution_steps.push(catalog.all_templates());
                }
                if tests {
                    execution_steps.push(catalog.all_tests());
                }
            }

            if let Some(tags) = tags {
                println!("Listing models with tags: {tags}");
            }
            if let Some(names) = names {
                let lookup_names = parse_cli_arg_list(&names);
                if models {
                    for name in &lookup_names {
                        execution_steps.push(catalog.lookup_model_by_name(name).unwrap());
                    }
                }
                if operations {
                    for name in &lookup_names {
                        execution_steps.push(catalog.lookup_operation_by_name(name).unwrap());
                    }
                }
                if transformations {
                    for name in &lookup_names {
                        execution_steps
                            .push(catalog.lookup_transformation_by_name(name).unwrap());
                    }
                }
                if templates {
                    for name in &lookup_names {
                        execution_steps.push(catalog.lookup_template_by_name(name).unwrap());
                    }
                }
                if tests {
                    for name in &lookup_names {
                        execution_steps.push(catalog.lookup_test_by_name(name).unwrap());
                    }
                }
            }

            for steps in execution_steps {
                if !json {
                    for step in steps {
                        println!("{}: {}", step.step_type(), step.name());
                    }
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &steps
                                .iter()
                                .map(|step| step.to_json())
                                .collect::<Vec<ExecutionStepJson>>()
                        )
                        .unwrap()
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Column, Model, OperationUsage, Test, Transformation};
    use std::path::PathBuf;

    #[test]
    fn build_transform_plan_includes_hook_steps() {
        let mut catalog = DataCatalog::new();
        let model: &'static Model = Box::leak(Box::new(Model {
            name: "m".into(),
            description: None,
            tags: None,
            table_id: None,
            managed: false,
            disabled: false,
            meta: None,
            default_transformation: Some("t".into()),
        }));
        let transformation: &'static Transformation = Box::leak(Box::new(Transformation {
            name: "t".into(),
            sql_code: "select 1".into(),
            model: "m".into(),
            dependent_tables: vec![],
            used_variables: None,
            template: None,
            columns: None,
            tests: None,
            pre_runs: Some(vec![
                OperationUsage {
                    name: "pre_op".into(),
                    props: None,
                    sql_code: "select pre".into(),
                },
            ]),
            post_runs: Some(vec![
                OperationUsage {
                    name: "post_op".into(),
                    props: None,
                    sql_code: "select post".into(),
                },
            ]),
            init_runs: None,
        }));
        let pre_op: &'static crate::entities::Operation = Box::leak(Box::new(
            crate::entities::Operation {
                name: "pre_op".into(),
                description: None,
                tags: None,
                sql_code: "select pre".into(),
                template: None,
                dependent_tables: vec![],
                used_variables: None,
            },
        ));
        let post_op: &'static crate::entities::Operation = Box::leak(Box::new(
            crate::entities::Operation {
                name: "post_op".into(),
                description: None,
                tags: None,
                sql_code: "select post".into(),
                template: None,
                dependent_tables: vec![],
                used_variables: None,
            },
        ));
        catalog.register_model(model);
        catalog.register_transformation(transformation);
        catalog.register_operation(pre_op);
        catalog.register_operation(post_op);

        let plan = build_transform_plan(
            &catalog,
            &RuntimeArgs {
                names: Some("m".into()),
                tags: None,
                vars: None,
                mappings: None,
                debug: false,
                json: false,
                init: false,
            },
        );
        let steps = plan.get_steps();
        assert_eq!(steps.len(), 1);
        let names: Vec<&str> = steps[0].iter().map(|step| step.name()).collect();
        assert_eq!(names, vec!["pre_op", "t", "post_op"]);
    }

    #[test]
    fn build_transform_plan_includes_init_hooks_when_requested() {
        let mut catalog = DataCatalog::new();
        let model: &'static Model = Box::leak(Box::new(Model {
            name: "m".into(),
            description: None,
            tags: None,
            table_id: None,
            managed: false,
            disabled: false,
            meta: None,
            default_transformation: Some("t".into()),
        }));
        let transformation: &'static Transformation = Box::leak(Box::new(Transformation {
            name: "t".into(),
            sql_code: "select 1".into(),
            model: "m".into(),
            dependent_tables: vec![],
            used_variables: None,
            template: None,
            columns: None,
            tests: None,
            pre_runs: None,
            post_runs: None,
            init_runs: Some(vec![OperationUsage {
                name: "init_op".into(),
                props: None,
                sql_code: "select init".into(),
            }]),
        }));
        let init_op: &'static crate::entities::Operation = Box::leak(Box::new(
            crate::entities::Operation {
                name: "init_op".into(),
                description: None,
                tags: None,
                sql_code: "select init".into(),
                template: None,
                dependent_tables: vec![],
                used_variables: None,
            },
        ));
        catalog.register_model(model);
        catalog.register_transformation(transformation);
        catalog.register_operation(init_op);

        let plan = build_transform_plan(
            &catalog,
            &RuntimeArgs {
                names: Some("m".into()),
                tags: None,
                vars: None,
                mappings: None,
                debug: false,
                json: false,
                init: true,
            },
        );
        let steps = plan.get_steps();
        assert_eq!(steps.len(), 1);
        let names: Vec<&str> = steps[0].iter().map(|step| step.name()).collect();
        assert_eq!(names, vec!["init_op", "t"]);
    }

    #[test]
    fn parsed_transformation_file_has_hooks() {
        let catalog = fixture_catalog_from_specs();
        let transformation = catalog
            .transformations_by_name
            .get("dummy_model_v1")
            .expect("dummy_model_v1");
        assert_eq!(
            transformation.pre_runs.as_ref().map(|runs| runs.len()),
            Some(2)
        );
        assert_eq!(
            transformation.post_runs.as_ref().map(|runs| runs.len()),
            Some(2)
        );
        assert_eq!(
            transformation.init_runs.as_ref().map(|runs| runs.len()),
            Some(1)
        );
    }

    #[test]
    fn parsed_transformation_file_has_tests() {
        let catalog = fixture_catalog_from_specs();
        let transformation = catalog
            .transformations_by_name
            .get("dummy_model_v1")
            .expect("dummy_model_v1");
        assert!(transformation.tests.as_ref().is_some_and(|t| !t.is_empty()));
    }

    fn fixture_catalog_from_specs() -> DataCatalog {
        let entities = crate::parser::parse_spec_dir(fixture_dir()).unwrap();
        let mut catalog = DataCatalog::new();
        for (_, entity) in entities {
            match entity {
                crate::entities::Entity::Config(c) => catalog.register_config(c),
                crate::entities::Entity::Model(m) => {
                    let leaked: &'static Model = Box::leak(Box::new(m));
                    catalog.register_model(leaked);
                }
                crate::entities::Entity::Transformation(t) => {
                    let leaked: &'static Transformation = Box::leak(Box::new(t));
                    catalog.register_transformation(leaked);
                }
                crate::entities::Entity::Template(t) => {
                    let leaked: &'static crate::entities::Template = Box::leak(Box::new(t));
                    catalog.register_template(leaked);
                }
                crate::entities::Entity::Test(t) => {
                    let leaked: &'static Test = Box::leak(Box::new(t));
                    catalog.register_test(leaked);
                }
                crate::entities::Entity::Operation(o) => {
                    let leaked: &'static crate::entities::Operation = Box::leak(Box::new(o));
                    catalog.register_operation(leaked);
                }
            }
        }
        catalog
    }

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../specs/data-specs")
    }

    #[test]
    fn build_apply_plan_resolves_operations_by_name() {
        let catalog = fixture_catalog_from_specs();
        let plan = build_apply_plan(
            &catalog,
            &RuntimeArgs {
                names: Some("dummy_operation".into()),
                tags: None,
                vars: None,
                mappings: None,
                debug: false,
                json: false,
                init: false,
            },
        );
        assert_eq!(plan.get_steps().len(), 1);
        assert_eq!(plan.get_steps()[0][0].name(), "dummy_operation");
    }

    #[test]
    fn build_test_plan_collects_model_tests() {
        let catalog = fixture_catalog_from_specs();
        let plan = build_test_plan(
            &catalog,
            &RuntimeArgs {
                names: Some("dummy_model".into()),
                tags: None,
                vars: None,
                mappings: None,
                debug: false,
                json: false,
                init: false,
            },
        );
        let steps = plan.get_steps();
        assert_eq!(steps.len(), 1);
        let names: Vec<&str> = steps[0].iter().map(|s| s.name()).collect();
        assert!(names.contains(&"dummy_test"));
        assert!(names.contains(&"dummy_test2"));
    }

    #[test]
    fn get_tests_for_model_includes_column_tests() {
        let mut catalog = DataCatalog::new();
        let model: &'static Model = Box::leak(Box::new(Model {
            name: "m".into(),
            description: None,
            tags: None,
            table_id: None,
            managed: false,
            disabled: false,
            meta: None,
            default_transformation: Some("t".into()),
        }));
        let transformation: &'static Transformation = Box::leak(Box::new(Transformation {
            name: "t".into(),
            sql_code: "select 1".into(),
            model: "m".into(),
            dependent_tables: vec![],
            used_variables: None,
            template: None,
            columns: Some(vec![Column {
                name: "c".into(),
                description: None,
                data_type: None,
                labels: None,
                tests: Some(vec!["col_test".into()]),
            }]),
            tests: Some(vec!["model_test".into()]),
            pre_runs: None,
            post_runs: None,
            init_runs: None,
        }));
        let model_test: &'static Test = Box::leak(Box::new(Test {
            name: "model_test".into(),
            description: None,
            sql_code: "select 1".into(),
            dependent_tables: vec![],
            used_variables: None,
            default_props: None,
        }));
        let col_test: &'static Test = Box::leak(Box::new(Test {
            name: "col_test".into(),
            description: None,
            sql_code: "select 2".into(),
            dependent_tables: vec![],
            used_variables: None,
            default_props: None,
        }));
        catalog.register_model(model);
        catalog.register_transformation(transformation);
        catalog.register_test(model_test);
        catalog.register_test(col_test);

        let steps = catalog.get_tests_for_model("m", None).unwrap();
        let names: Vec<&str> = steps.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["col_test", "model_test"]);
    }
}
