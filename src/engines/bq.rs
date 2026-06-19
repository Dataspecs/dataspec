use crate::context::ctx::Ctx;
use crate::engines::common::ExecutionStatistics;
use crate::engines::DbEngine;
use gcp_bigquery_client::model::query_request::QueryRequest;
use gcp_bigquery_client::Client;
use std::error::Error;

pub struct BQEngine {
    client: Client,
    project_id: String,
}

impl DbEngine for BQEngine {
    async fn init(ctx: &Ctx<'_>) -> Result<Box<Self>, Box<dyn Error>> {
        let service_account_path = ctx
            .props
            .as_ref()
            .ok_or("properties not found")?
            .get("service_account_path");

        let client = if let Some(path) = service_account_path {
            Client::from_service_account_key_file(path).await?
        } else {
            Client::from_application_default_credentials().await?
        };

        let project_id = ctx
            .props
            .as_ref()
            .ok_or("properties not found")?
            .get("project_id")
            .ok_or("project_id not found in properties")?;

        Ok(Box::new(BQEngine {
            client,
            project_id: project_id.to_string(),
        }))
    }

    async fn execute(&self, sql: &str) -> Result<ExecutionStatistics, Box<dyn Error>> {
        let query_request = QueryRequest::new(sql);
        let rs = self
            .client
            .job()
            .query(&self.project_id, query_request)
            .await?;
        let job_reference = rs
            .query_response()
            .job_reference
            .as_ref()
            .ok_or("Job reference is not available")?;

        let location = job_reference.location.as_ref().map(|x| x.as_str());
        let job = self.client.job().get_job(
            job_reference
                .project_id
                .as_deref()
                .ok_or("project_id not found")?,
            job_reference.job_id.as_deref().ok_or("job_id not found")?,
            location,
        ).await?;

        let stats = job
            .statistics
            .and_then(|s| s.query)
            .ok_or("Stats are not available")?;

        Ok(ExecutionStatistics {
            total_bytes_processed: stats
                .total_bytes_billed
                .as_ref()
                .and_then(|b| b.parse::<i64>().ok()),
            num_dml_affected_rows: stats
                .num_dml_affected_rows
                .and_then(|s| s.parse::<i64>().ok()),
            cache_hit: stats.cache_hit,
            bytes_billed: stats
                .total_bytes_billed
                .as_ref()
                .and_then(|b| b.parse::<i64>().ok()),
        })
    }
}
