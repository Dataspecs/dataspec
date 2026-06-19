use crate::context::ctx::Ctx;
use crate::entities::{ExecutionPlan, ExecutionStepJson};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::error::Error;

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
    async fn init(ctx: &Ctx<'_>) -> Result<Box<Self>, Box<dyn Error>>
    where
        Self: Sized;
    async fn execute(&self, sql: &str) -> Result<ExecutionStatistics, Box<dyn Error>>;

    async fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        ctx: &Ctx<'_>,
    ) -> Result<ExecutionPlanResults, Box<dyn Error>> {
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

        for steps in plan.get_steps() {
            for step in steps {
                tracing::info!("Execute {}", step.name());
                tracing::debug!("Start rendering SQL for step: {}", step.name());
                let sql = subst::substitute(step.sql(), ctx).unwrap_or_default();
                tracing::debug!("Executing SQL: {sql}");

                let statistics = self.execute(&sql).await?;
                plan_results.step_results.push(ExecutionPlanStepResult {
                    step: ExecutionStepJson {
                        name: step.name().to_string(),
                        sql,
                        step_type: step.step_type(),
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
