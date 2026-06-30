use crate::context::ctx::Ctx;
use crate::engines::common::ExecutionStatistics;
use crate::engines::DbEngine;
use std::error::Error;
use tokio_postgres::NoTls;

pub struct PgEngine {
    client: tokio_postgres::Client,
}

impl DbEngine for PgEngine {
    async fn init(ctx: &Ctx<'_>) -> Result<Box<Self>, Box<dyn Error>> {
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

    async fn execute(&self, sql: &str) -> Result<ExecutionStatistics, Box<dyn Error>> {
        let stmt = self.client.prepare(sql).await?;
        let rows_affected = self.client.execute(&stmt, &[]).await?;

        Ok(ExecutionStatistics {
            total_bytes_processed: None,
            num_dml_affected_rows: Some(rows_affected as i64),
            cache_hit: None,
            bytes_billed: None,
        })
    }
}
