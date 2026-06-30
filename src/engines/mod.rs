#[cfg(feature = "bq")]
pub mod bq;
pub mod common;
pub mod dryrun;
#[cfg(feature = "pg")]
pub mod pg;

pub use common::{DbEngine, ExecutionPlanResults, ExecutionStatistics};

use crate::context::ctx::Ctx;
use crate::entities::ExecutionPlan;
use std::error::Error;

pub enum Engine {
    DryRun(dryrun::DryRunEngine),
    #[cfg(feature = "bq")]
    BQ(bq::BQEngine),
    #[cfg(feature = "pg")]
    PG(pg::PgEngine),
}

impl Engine {
    pub async fn from_provider(ctx: &Ctx<'_>) -> Result<Self, Box<dyn Error>> {
        let provider = ctx
            .config_props()
            .get("provider")
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "dryrun".to_string());

        match provider.as_str() {
            "bq" => {
                #[cfg(feature = "bq")]
                {
                    let eng = bq::BQEngine::init(ctx).await?;
                    Ok(Engine::BQ(*eng))
                }
                #[cfg(not(feature = "bq"))]
                {
                    Err("BQ engine requested but 'bq' feature is not enabled. Build with --features bq".into())
                }
            }
            "pg" | "postgres" | "postgresql" => {
                #[cfg(feature = "pg")]
                {
                    let eng = pg::PgEngine::init(ctx).await?;
                    Ok(Engine::PG(*eng))
                }
                #[cfg(not(feature = "pg"))]
                {
                    Err("PostgreSQL engine requested but 'pg' feature is not enabled. Build with --features pg".into())
                }
            }
            _ => {
                let eng = dryrun::DryRunEngine::init(ctx).await?;
                Ok(Engine::DryRun(*eng))
            }
        }
    }

    pub async fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        ctx: &Ctx<'_>,
    ) -> Result<ExecutionPlanResults, Box<dyn Error>> {
        match self {
            Engine::DryRun(e) => e.execute_plan(plan, ctx).await,
            #[cfg(feature = "bq")]
            Engine::BQ(e) => e.execute_plan(plan, ctx).await,
            #[cfg(feature = "pg")]
            Engine::PG(e) => e.execute_plan(plan, ctx).await,
        }
    }
}
