use crate::context::ctx::Ctx;
use crate::engines::common::ExecutionStatistics;
use crate::engines::DbEngine;
use std::error::Error;

pub struct DryRunEngine;

impl DbEngine for DryRunEngine {
    async fn init(_ctx: &Ctx<'_>) -> Result<Box<Self>, Box<dyn Error>> {
        Ok(Box::new(DryRunEngine))
    }

    async fn execute(&self, _sql: &str) -> Result<ExecutionStatistics, Box<dyn Error>> {
        Ok(ExecutionStatistics {
            total_bytes_processed: Some(1),
            num_dml_affected_rows: Some(0),
            cache_hit: Some(false),
            bytes_billed: Some(0),
        })
    }
}
