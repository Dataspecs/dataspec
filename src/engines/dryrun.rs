use crate::context::ctx::Ctx;
use crate::engines::common::ExecutionStatistics;
use crate::engines::DbEngine;
use std::error::Error;

pub struct DryRunEngine;

impl DbEngine for DryRunEngine {
    fn init(_ctx: &Ctx<'_>) -> impl std::future::Future<Output = Result<Box<Self>, Box<dyn Error>>> + Send {
        async move { Ok(Box::new(DryRunEngine)) }
    }

    fn execute(
        &self,
        sql: &str,
    ) -> impl std::future::Future<Output = Result<ExecutionStatistics, Box<dyn Error>>> + Send {
        async move {
            tracing::info!("{sql}");
            Ok(ExecutionStatistics {
                total_bytes_processed: Some(1),
                num_dml_affected_rows: Some(0),
                cache_hit: Some(false),
                bytes_billed: Some(0),
            })
        }
    }
}
