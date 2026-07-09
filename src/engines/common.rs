use crate::context::{render_runtime, render_runtime_step, Ctx};
use crate::entities::{ExecutionPlan, ExecutionStepJson};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::error::Error;
use std::future::Future;

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionStatistics {
    pub total_bytes_processed: Option<i64>,
    pub num_dml_affected_rows: Option<i64>,
    pub cache_hit: Option<bool>,
    pub bytes_billed: Option<i64>,
}

#[derive(Serialize)]
pub struct ExecutionPlanStepResult {
    pub step: ExecutionStepJson,
    pub result: ExecutionStatistics,
}

#[derive(Serialize)]
pub struct ExecutionPlanResults {
    pub result: ExecutionStatistics,
    pub step_results: Vec<ExecutionPlanStepResult>,
    pub session_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

pub trait DbEngine {
    fn init(ctx: &Ctx<'_>) -> impl Future<Output = Result<Box<Self>, Box<dyn Error>>> + Send
    where
        Self: Sized;

    fn execute(
        &self,
        sql: &str,
    ) -> impl Future<Output = Result<ExecutionStatistics, Box<dyn Error>>> + Send;

    fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        ctx: &Ctx<'_>,
    ) -> impl Future<Output = Result<ExecutionPlanResults, Box<dyn Error>>> + Send
    where
        Self: Sync,
    {
        let all_steps: Vec<Vec<(ExecutionStepJson, bool, Option<std::collections::HashMap<String, String>>)>> =
            plan
                .get_steps()
                .iter()
                .map(|steps| {
                    steps
                        .iter()
                        .map(|step| {
                            (
                                step.to_json(),
                                step.is_hook_operation(),
                                step.runtime_props().cloned(),
                            )
                        })
                        .collect()
                })
                .collect();

        async move {
            let mut plan_results = ExecutionPlanResults {
                result: ExecutionStatistics {
                    total_bytes_processed: Some(0),
                    num_dml_affected_rows: Some(0),
                    cache_hit: Some(false),
                    bytes_billed: Some(0),
                },
                step_results: Vec::new(),
                session_id: ctx.session_id.clone(),
                start_time: Utc::now(),
                end_time: Utc::now(),
            };

            for steps in all_steps {
                for (mut step, is_hook_operation, step_props) in steps {
                    if is_hook_operation {
                        step.sql = resolve_hook_sql(&step.name, ctx);
                    }
                    tracing::info!("Execute {}", step.name);
                    tracing::debug!("Start rendering SQL for step: {}", step.name);
                    let sql = if is_hook_operation {
                        render_runtime_step(&step.sql, ctx, step_props.as_ref())
                    } else {
                        render_runtime(&step.sql, ctx)
                    };
                    tracing::debug!("Executing SQL: {sql}");

                    let statistics = self.execute(&sql).await?;
                    plan_results.step_results.push(ExecutionPlanStepResult {
                        step: ExecutionStepJson {
                            name: step.name,
                            sql,
                            step_type: step.step_type,
                        },
                        result: statistics.clone(),
                    });

                    plan_results.result.total_bytes_processed = Some(
                        plan_results.result.total_bytes_processed.unwrap_or(0)
                            + statistics.total_bytes_processed.unwrap_or(0),
                    );
                    plan_results.result.num_dml_affected_rows = Some(
                        plan_results.result.num_dml_affected_rows.unwrap_or(0)
                            + statistics.num_dml_affected_rows.unwrap_or(0),
                    );
                    plan_results.result.cache_hit = Some(
                        plan_results.result.cache_hit.unwrap_or(false)
                            || statistics.cache_hit.unwrap_or(false),
                    );
                    plan_results.result.bytes_billed = Some(
                        plan_results.result.bytes_billed.unwrap_or(0)
                            + statistics.bytes_billed.unwrap_or(0),
                    );
                }
            }
            plan_results.end_time = Utc::now();

            Ok(plan_results)
        }
    }
}

fn resolve_hook_sql(operation_name: &str, ctx: &Ctx<'_>) -> String {
    let catalog = ctx
        .data_catalog
        .expect("data_catalog must be set before executing hook operations");
    catalog
        .operations_by_name
        .get(operation_name)
        .unwrap_or_else(|| panic!("Can't find operation {operation_name}"))
        .sql_code
        .clone()
}
