use std::collections::HashMap;
use std::env;

use clap::{Parser, Subcommand};

use crate::context::ctx::Ctx;
use crate::engines::Engine;
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

async fn execute_plan(
    execution_plan: &ExecutionPlan,
    ctx: &Ctx<'_>,
    level: tracing::level_filters::LevelFilter,
    json_output: bool,
) {
    let engine = match Engine::from_provider(ctx).await {
        Ok(eng) => eng,
        Err(e) => {
            tracing::error!("Failed to init engine: {e}");
            std::process::exit(1);
        }
    };

    tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(false)
        .with_target(false)
        .with_thread_names(false)
        .with_max_level(level)
        .init();

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
        Err(e) => {
            tracing::error!("Execution failed: {e}");
            std::process::exit(1);
        }
    }
    tracing::info!("Execution plan completed");
}

/// Runtime CLI handler for generated data-spec projects.
pub async fn spec_handler(catalog: &DataCatalog) {
    let cli = Cli::parse();

    let mut execution_plan = ExecutionPlan::new();
    let mut ctx = Ctx::new();

    match cli.command {
        Commands::Transform {
            names,
            tags,
            vars,
            mappings,
            debug,
            json,
        } => {
            let level = if json {
                tracing::level_filters::LevelFilter::OFF
            } else if debug {
                tracing::level_filters::LevelFilter::DEBUG
            } else {
                tracing::level_filters::LevelFilter::INFO
            };

            let cli_vars = vars
                .as_deref()
                .map(parse_key_value_pairs)
                .unwrap_or_default();
            let table_mappings = mappings
                .as_deref()
                .map(parse_key_value_pairs)
                .unwrap_or_default();

            ctx.set_vars(cli_vars);
            ctx.set_env_vars(env::vars().collect());
            ctx.set_table_mappings(table_mappings);
            ctx.set_data_catalog(catalog);

            if let Some(tags) = tags {
                let lookup_tags: Vec<String> = tags.split(',').map(|s| s.to_string()).collect();
                if let Some(steps) = catalog.lookup_models_by_tags(lookup_tags) {
                    execution_plan.add_steps(steps);
                }
            }
            if let Some(names) = names {
                let lookup_names: Vec<String> = names.split(',').map(|s| s.to_string()).collect();
                let plan: Vec<Vec<Box<dyn ExecutionStep>>> = lookup_names
                    .iter()
                    .map(|name| {
                        let model_name = if name.contains("::") {
                            name.split("::").next().unwrap()
                        } else {
                            name.as_str()
                        };

                        let transformation_name = if name.contains("::") {
                            Some(name.split("::").nth(1).unwrap())
                        } else {
                            None
                        };
                        catalog
                            .get_execution_pipeline_by_model_name(model_name, transformation_name)
                            .ok_or_else(|| format!("Can't find model {name}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .expect("Failed to find all models");

                for steps in plan {
                    execution_plan.add_steps(steps);
                }
            }
            execute_plan(&execution_plan, &ctx, level, json).await;
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
            let level = if json {
                tracing::level_filters::LevelFilter::OFF
            } else {
                tracing::level_filters::LevelFilter::INFO
            };

            tracing_subscriber::fmt()
                .with_file(false)
                .with_line_number(false)
                .with_target(false)
                .with_thread_names(false)
                .with_max_level(level)
                .init();

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
                let lookup_names: Vec<String> = names.split(',').map(|s| s.to_string()).collect();
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
