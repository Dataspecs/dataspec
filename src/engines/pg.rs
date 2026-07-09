use crate::context::ctx::Ctx;
use crate::engines::common::ExecutionStatistics;
use crate::engines::DbEngine;
use std::error::Error;
use tokio_postgres::NoTls;

pub struct PgEngine {
    client: tokio_postgres::Client,
}

impl DbEngine for PgEngine {
    fn init(ctx: &Ctx<'_>) -> impl std::future::Future<Output = Result<Box<Self>, Box<dyn Error>>> + Send {
        async move {
            let connection_string = ctx
                .config_props()
                .get("connection_string")
                .map(|s| s.as_str())
                .ok_or(
                    "connection_string not found in config props. \
                     Set connection_string in config (e.g. postgres://user:password@localhost:5432/dbname)",
                )?;

            let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    tracing::error!("PostgreSQL connection error: {e}");
                }
            });

            Ok(Box::new(PgEngine { client }))
        }
    }

    fn execute(
        &self,
        sql: &str,
    ) -> impl std::future::Future<Output = Result<ExecutionStatistics, Box<dyn Error>>> + Send {
        async move {
            let stmt = self.client.prepare(sql).await?;
            tracing::debug!("Executing prepared statement");
            let rows_affected = self.client.execute(&stmt, &[]).await?;
            tracing::debug!("Rows affected: {rows_affected}");

            Ok(ExecutionStatistics {
                total_bytes_processed: None,
                num_dml_affected_rows: Some(rows_affected as i64),
                cache_hit: None,
                bytes_billed: None,
            })
        }
    }
}
