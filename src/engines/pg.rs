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
            if is_read_query(sql) {
                let rows = self.client.query(sql, &[]).await?;
                tracing::debug!("Rows returned: {}", rows.len());

                return Ok(ExecutionStatistics {
                    total_bytes_processed: None,
                    num_dml_affected_rows: None,
                    num_rows: Some(rows.len() as i64),
                    cache_hit: None,
                    bytes_billed: None,
                });
            }

            let stmt = self.client.prepare(sql).await?;
            tracing::debug!("Executing prepared statement");
            let rows_affected = self.client.execute(&stmt, &[]).await?;
            tracing::debug!("Rows affected: {rows_affected}");

            Ok(ExecutionStatistics {
                total_bytes_processed: None,
                num_dml_affected_rows: Some(rows_affected as i64),
                num_rows: None,
                cache_hit: None,
                bytes_billed: None,
            })
        }
    }
}

fn is_read_query(sql: &str) -> bool {
    let trimmed = sql.trim_start();
    trimmed
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("select"))
        || trimmed
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("with"))
}
