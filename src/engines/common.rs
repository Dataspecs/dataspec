use crate::context::{render_runtime, render_runtime_step, Ctx};
use crate::entities::{ExecutionPlan, ExecutionStepJson, ExecutionStepType};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::error::Error;
use std::future::Future;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Success,
    Failed,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionStatistics {
    pub total_bytes_processed: Option<i64>,
    pub num_dml_affected_rows: Option<i64>,
    pub num_rows: Option<i64>,
    pub cache_hit: Option<bool>,
    pub bytes_billed: Option<i64>,
}

impl Default for ExecutionStatistics {
    fn default() -> Self {
        Self {
            total_bytes_processed: Some(0),
            num_dml_affected_rows: Some(0),
            num_rows: None,
            cache_hit: Some(false),
            bytes_billed: Some(0),
        }
    }
}

#[derive(Serialize)]
pub struct ExecutionPlanStepResult {
    pub step: ExecutionStepJson,
    pub result: ExecutionStatistics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_status: Option<TestStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn evaluate_test_status(num_rows: Option<i64>) -> TestStatus {
    if num_rows.unwrap_or(0) == 0 {
        TestStatus::Success
    } else {
        TestStatus::Failed
    }
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
        self.run_plan(plan, ctx, false)
    }

    fn execute_test_plan(
        &self,
        plan: &ExecutionPlan,
        ctx: &Ctx<'_>,
    ) -> impl Future<Output = Result<ExecutionPlanResults, Box<dyn Error>>> + Send
    where
        Self: Sync,
    {
        self.run_plan(plan, ctx, true)
    }

    fn run_plan(
        &self,
        plan: &ExecutionPlan,
        ctx: &Ctx<'_>,
        is_test_run: bool,
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
                result: ExecutionStatistics::default(),
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

                    match self.execute(&sql).await {
                        Ok(statistics) => {
                            let test_status = if is_test_run && step.step_type == ExecutionStepType::Test
                            {
                                Some(evaluate_test_status(statistics.num_rows))
                            } else {
                                None
                            };
                            plan_results.step_results.push(ExecutionPlanStepResult {
                                step: ExecutionStepJson {
                                    name: step.name,
                                    sql,
                                    step_type: step.step_type,
                                },
                                result: statistics.clone(),
                                test_status,
                                error: None,
                            });
                            accumulate_statistics(&mut plan_results.result, &statistics);
                        }
                        Err(e) => {
                            if is_test_run && step.step_type == ExecutionStepType::Test {
                                plan_results.step_results.push(ExecutionPlanStepResult {
                                    step: ExecutionStepJson {
                                        name: step.name,
                                        sql,
                                        step_type: step.step_type,
                                    },
                                    result: ExecutionStatistics::default(),
                                    test_status: Some(TestStatus::Error),
                                    error: Some(e.to_string()),
                                });
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }
            }
            plan_results.end_time = Utc::now();

            Ok(plan_results)
        }
    }
}

fn accumulate_statistics(total: &mut ExecutionStatistics, statistics: &ExecutionStatistics) {
    total.total_bytes_processed = Some(
        total.total_bytes_processed.unwrap_or(0)
            + statistics.total_bytes_processed.unwrap_or(0),
    );
    total.num_dml_affected_rows = Some(
        total.num_dml_affected_rows.unwrap_or(0)
            + statistics.num_dml_affected_rows.unwrap_or(0),
    );
    total.cache_hit = Some(
        total.cache_hit.unwrap_or(false) || statistics.cache_hit.unwrap_or(false),
    );
    total.bytes_billed = Some(
        total.bytes_billed.unwrap_or(0) + statistics.bytes_billed.unwrap_or(0),
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_test_status_zero_rows_is_success() {
        assert_eq!(evaluate_test_status(Some(0)), TestStatus::Success);
        assert_eq!(evaluate_test_status(None), TestStatus::Success);
    }

    #[test]
    fn evaluate_test_status_positive_rows_is_failed() {
        assert_eq!(evaluate_test_status(Some(1)), TestStatus::Failed);
        assert_eq!(evaluate_test_status(Some(42)), TestStatus::Failed);
    }
}
