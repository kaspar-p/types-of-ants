use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use stdext::function_name;

use ant_library::db::{Database, DatabaseConfig, TypesOfAntsDatabase, database_connection};
use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use ant_library::host_architecture::HostArchitecture;
use tokio_postgres::Row;

#[derive(Clone)]
pub struct AntZooStorageClient {
    db: Database,
}

#[async_trait]
impl TypesOfAntsDatabase for AntZooStorageClient {
    async fn connect(config: &DatabaseConfig) -> Result<Self, anyhow::Error> {
        let database = database_connection(&config).await?;

        Ok(AntZooStorageClient { db: database })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostGroupHost {
    pub name: String,
    pub arch: HostArchitecture,
    pub added_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostGroup {
    pub id: String,
    pub name: String,
    pub environment: String,
    pub description: Option<String>,
    pub hosts: Vec<HostGroupHost>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentJob {
    pub job_id: String,
    pub project_id: String,
    pub deployment_pipeline_id: String,
    pub revision: String,
    pub target_type: String,
    pub target_id: String,
    pub event_name: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub deployment_id: String,
    pub revision_id: String,
    pub target_type: String,
    pub target_id: String,
    pub event_name: String,
    pub created_at: DateTime<Utc>,
}

impl AntZooStorageClient {
    pub async fn register_project(
        &self,
        project: &str,
        is_owned: bool,
    ) -> Result<(), anyhow::Error> {
        let con = self.db.get().await?;

        con.execute(
            "
            insert into project
                (project_id, owned)
            values
                ($1, $2)
            ",
            &[&project, &is_owned],
        )
        .await?;

        Ok(())
    }

    pub async fn get_project(&self, project: &str) -> Result<bool, anyhow::Error> {
        let con = self.db.get().await?;

        let row = con
            .query_opt(
                "
            select
                (project_id, owned)
            from project
            where
                project_id = $1
            ",
                &[&project],
            )
            .await?;

        Ok(row.is_some())
    }

    pub async fn get_project_from_deployment_pipeline(
        &self,
        deployment_pipeline_id: &str,
    ) -> Result<String, anyhow::Error> {
        let project_id = self
            .db
            .get()
            .await?
            .query_one(
                "
            select project_id
            from deployment_pipeline
            where
                deployment_pipeline_id = $1
            ",
                &[&deployment_pipeline_id],
            )
            .await?
            .get("project_id");

        Ok(project_id)
    }

    /// Returns (revision_id, is_new)
    pub async fn upsert_revision(
        &self,
        project: &str,
        version: &str,
    ) -> Result<(String, bool), anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let revision_id: Option<String> = tx
            .query_opt(
                "
                select revision_id
                from revision
                where
                    project_id = $1 and
                    deployment_version = $2
            ",
                &[&project, &version],
            )
            .await?
            .map(|r| r.get("revision_id"));

        match revision_id {
            Some(id) => return Ok((id, false)),
            None => {
                let revision_id: String = tx
                    .query_one(
                        "
                    insert into revision
                        (project_id, deployment_version)
                    values
                        ($1, $2)
                    returning revision_id
                    ",
                        &[&project, &version],
                    )
                    .await?
                    .get("revision_id");

                let pipeline_id = self
                    .get_deployment_pipeline_by_project(project)
                    .await?
                    .unwrap();

                tx.execute(
                    "
                insert into deployment
                    (revision_id, target_type, target_id, event_name)
                values
                    ($1, 'pipeline', $2, 'pipeline-started')
                ",
                    &[&revision_id, &pipeline_id],
                )
                .await?;

                tx.commit().await?;

                return Ok((revision_id, true));
            }
        }
    }

    /// Returns (artifact id, version, local path)
    pub async fn get_artifact_by_revision(
        &self,
        arch: Option<&HostArchitecture>,
        revision_id: &str,
    ) -> Result<Option<(String, String, PathBuf)>, anyhow::Error> {
        let con = self.db.get().await?;

        let exists = con
            .query_opt(
                "
            select artifact_id, deployment_version, local_path
            from artifact
                join revision on revision.revision_id
                    = artifact.revision_id
            where
                revision.revision_id = $1 and
                artifact.architecture_id = $2
            ",
                &[&revision_id, &arch.map(|a| a.as_str())],
            )
            .await?
            .map(|row| -> Result<(String, String, PathBuf), anyhow::Error> {
                Ok((
                    row.get("artifact_id"),
                    row.get("deployment_version"),
                    PathBuf::from_str(row.get("local_path"))?,
                ))
            })
            .transpose()?;

        Ok(exists)
    }

    pub async fn missing_artifacts_for_revision_id(
        &self,
        revision_id: &str,
    ) -> Result<Vec<HostArchitecture>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let rows = tx
            .query(
                "
            select architecture.architecture_id
            from architecture
            where architecture.architecture_id not in (
                select artifact.architecture_id
                from artifact
                where artifact.revision_id = $1
            )
            ",
                &[&revision_id],
            )
            .await?;

        let architectures = rows
            .iter()
            .map(|row| HostArchitecture::from_str(row.get("architecture_id")))
            .collect::<Result<Vec<HostArchitecture>, anyhow::Error>>()?;

        Ok(architectures)
    }

    pub async fn missing_artifacts_for_project_version(
        &self,
        project: &str,
        version: &str,
    ) -> Result<Vec<HostArchitecture>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let revision: String = tx
            .query_one(
                "
            select revision_id
            from revision
            where
                project_id = $1 and
                deployment_version = $2",
                &[&project, &version],
            )
            .await?
            .get("revision_id");

        return Ok(self.missing_artifacts_for_revision_id(&revision).await?);
    }

    pub async fn list_architectures(&self) -> Result<Vec<HostArchitecture>, anyhow::Error> {
        let architectures = self
            .db
            .get()
            .await?
            .query("select architecture_id from architecture", &[])
            .await
            .with_context(|| format!("{}", function_name!()))?
            .iter()
            .map(|row| HostArchitecture::from_str(row.get("architecture_id")))
            .collect::<Result<Vec<HostArchitecture>, anyhow::Error>>()?;

        Ok(architectures)
    }

    pub async fn register_artifact(
        &self,
        revision_id: &str,
        arch: Option<&HostArchitecture>,
        path: &Path,
    ) -> Result<String, anyhow::Error> {
        let mut con = self.db.get().await?;

        let tx = con.transaction().await?;

        let artifact_id = tx
            .query_one(
                "
            insert into artifact
                (revision_id, architecture_id, local_path)
            values
                ($1, $2, $3)
            returning artifact_id
            ",
                &[
                    &revision_id,
                    &arch.map(|a| a.as_str()),
                    &path
                        .as_os_str()
                        .to_str()
                        .expect(&format!("bad artifact path: {}", path.display())),
                ],
            )
            .await?
            .get("artifact_id");

        tx.commit().await?;

        Ok(artifact_id)
    }

    pub async fn update_artifact(&self, artifact_id: &str) -> Result<(), anyhow::Error> {
        let con = self.db.get().await?;

        con.execute(
            "
            update artifact
            set
                updated_at = now()
            where
                artifact_id = $1
            ",
            &[&artifact_id],
        )
        .await?;

        Ok(())
    }

    pub async fn register_new_secret_version(
        &self,
        secret_name: &str,
        secret_environment: &str,
        valid_for: chrono::Duration,
        secret_value: &[u8],
    ) -> Result<String, anyhow::Error> {
        let existing_version: Option<i32> = self
            .db
            .get()
            .await?
            .query_opt(
                "
        select secret_version
        where
            secret_name = $1 and
            secret_environment = $2 and
            deleted_at is null
        order by created_at desc
        ",
                &[&secret_name, &secret_environment],
            )
            .await?
            .map(|row| row.get("secret_version"));

        let row = self
            .db
            .get()
            .await?
            .query_one(
                "
        insert into secret(
            secret_name,
            secret_environment,
            secret_version,
            valid_for_seconds,
            secret_value
        )
        values
            ($1, $2, $3, $4, $5)
        returning secret_id
        ",
                &[
                    &secret_name,
                    &secret_environment,
                    &existing_version.unwrap_or(1),
                    &valid_for.num_seconds(),
                    &secret_value,
                ],
            )
            .await?;

        let secret_id: String = row.get("secret_id");
        Ok(secret_id)
    }

    pub async fn get_deployment_pipeline_by_project(
        &self,
        project: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let exists = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select deployment_pipeline_id
            from deployment_pipeline
            where project_id = $1
            limit 1
            ",
                &[&project],
            )
            .await?
            .map(|r| r.get("deployment_pipeline_id"));

        Ok(exists)
    }

    /// Returns (stage_name, stage_id, stage_type)
    pub async fn get_deployment_pipeline_stages(
        &self,
        project: &str,
    ) -> Result<Vec<(String, String, String)>, anyhow::Error> {
        let rows = self
            .db
            .get()
            .await?
            .query(
                "
            select
                stage_name,
                deployment_pipeline_stage_id,
                stage_type
            from deployment_pipeline_stage
                join deployment_pipeline on deployment_pipeline_stage.deployment_pipeline_id
                    = deployment_pipeline.deployment_pipeline_id
            where
                deployment_pipeline.project_id = $1
            order by stage_order asc
        ",
                &[&project],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), project))?;

        let stages = rows
            .iter()
            .map(|row| -> (String, String, String) {
                (
                    row.get("stage_name"),
                    row.get("deployment_pipeline_stage_id"),
                    row.get("stage_type"),
                )
            })
            .collect();

        Ok(stages)
    }

    /// Returns (pipeline_id, stage_name, stage_order, stage_type)
    pub async fn get_deployment_pipeline_stage(
        &self,
        stage_id: &str,
    ) -> Result<Option<(String, String, i32, String)>, anyhow::Error> {
        let stage = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select deployment_pipeline_id, stage_name, stage_order, stage_type
            from deployment_pipeline_stage
            where
                deployment_pipeline_stage_id = $1
        ",
                &[&stage_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), stage_id))?
            .map(|row| {
                (
                    row.get("deployment_pipeline_id"),
                    row.get("stage_name"),
                    row.get("stage_order"),
                    row.get("stage_type"),
                )
            });

        Ok(stage)
    }

    pub async fn get_deployment_pipeline_stage_by_order(
        &self,
        deployment_pipeline_id: &str,
        order: i32,
    ) -> Result<Option<String>, anyhow::Error> {
        let stage_id = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select deployment_pipeline_stage_id
            from deployment_pipeline_stage
                join deployment_pipeline on deployment_pipeline.deployment_pipeline_id
                    = deployment_pipeline_stage.deployment_pipeline_id
            where
                deployment_pipeline.deployment_pipeline_id = $1 and
                stage_order = $2
            ",
                &[&deployment_pipeline_id, &order],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), deployment_pipeline_id, order))?
            .map(|r| r.get("deployment_pipeline_stage_id"));

        Ok(stage_id)
    }

    pub async fn get_deployment_pipeline_stage_by_host_group(
        &self,
        host_group_id: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let stage_id = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select deployment_pipeline_stage_id
            from deployment_pipeline_stage
            where
                stage_type_deploy_host_group_id = $1
            ",
                &[&host_group_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), host_group_id))?
            .map(|r| r.get("deployment_pipeline_stage_id"));

        Ok(stage_id)
    }

    pub async fn delete_deployment_pipeline_stage(
        &self,
        stage_id: &str,
    ) -> Result<(), anyhow::Error> {
        let deleted = self
            .db
            .get()
            .await?
            .execute(
                "
            delete from deployment_pipeline_stage
            where deployment_pipeline_stage_id = $1
            ",
                &[&stage_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), stage_id))?;

        if deleted != 1 {
            panic!(
                "Deleted {deleted} stages '{stage_id}' but meant to just delete a single stage!"
            );
        }

        Ok(())
    }

    pub async fn create_deployment_pipeline_deployment_stage(
        &self,
        deployment_pipeline_id: &str,
        stage_name: &str,
        host_group_id: &str,
        ordering: i32,
    ) -> Result<String, anyhow::Error> {
        let stage_id: String = self
            .db
            .get()
            .await?
            .query_one(
                "
            insert into deployment_pipeline_stage
            (
                deployment_pipeline_id,
                stage_name,
                stage_type,
                stage_type_deploy_host_group_id,
                stage_order
            )
            values
                ($1, $2, 'deploy', $3, $4)
            returning deployment_pipeline_stage_id
        ",
                &[
                    &deployment_pipeline_id,
                    &stage_name,
                    &host_group_id,
                    &ordering,
                ],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: {} {} {} {}",
                    function_name!(),
                    deployment_pipeline_id,
                    stage_name,
                    host_group_id,
                    ordering
                )
            })?
            .get("deployment_pipeline_stage_id");

        Ok(stage_id)
    }

    pub async fn get_deployment(
        &self,
        revision_id: &str,
        target_type: &str,
        target_id: &str,
        event_name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let deployment_id = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select deployment_id
            from deployment
            where
                revision_id = $1 and
                target_type = $2 and
                target_id = $3 and
                event_name = $4
            ",
                &[&revision_id, &target_type, &target_id, &event_name],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: {} {} {} {}",
                    function_name!(),
                    revision_id,
                    target_type,
                    target_id,
                    event_name
                )
            })?
            .map(|row| row.get("deployment_id"));

        Ok(deployment_id)
    }

    pub async fn start_deployment_job(&self, deployment_job_id: &str) -> Result<(), anyhow::Error> {
        self.db
            .get()
            .await?
            .execute(
                "
            update deployment_job
            set started_at = now()
            where deployment_job_id = $1
            ",
                &[&deployment_job_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), deployment_job_id))?;

        Ok(())
    }

    pub async fn unstart_deployment_job(
        &self,
        deployment_job_id: &str,
    ) -> Result<(), anyhow::Error> {
        self.db
            .get()
            .await?
            .execute(
                "
            update deployment_job
            set started_at = null
            where deployment_job_id = $1
            ",
                &[&deployment_job_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), deployment_job_id))?;

        Ok(())
    }

    pub async fn set_deployment_job_retryable(
        &self,
        deployment_job_id: &str,
        retryable: bool,
    ) -> Result<(), anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        // Mark deployment job is_retryable
        tx.query_one(
            "
            update deployment_job
            set
                is_retryable = $1,
                updated_at = now()
            where
                deployment_job_id = $2
            returning deployment_job_id
            ",
            &[&retryable, &deployment_job_id],
        )
        .await
        .with_context(|| format!("{}: {} {}", function_name!(), deployment_job_id, retryable))?;

        tx.commit().await?;

        Ok(())
    }

    /// Returns Some(created Deployment ID) if the job was successful, None if it wasn't,
    /// since there was no deployment resulting of it.
    pub async fn complete_deployment_job(
        &self,
        deployment_job_id: &str,
        revision_id: &str,
        target_type: &str,
        target_id: &str,
        event_name: &str,
        is_success: bool,
    ) -> Result<Option<String>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        // Mark deployment job complete
        tx.execute(
            "
            update deployment_job
            set
                is_success = $1,
                is_retryable = false,
                finished_at = now(),
                updated_at = now()
            where
                deployment_job_id = $2
            ",
            &[&is_success, &deployment_job_id],
        )
        .await
        .with_context(|| {
            format!(
                "{}: mark complete {} {} {} {} {} {}",
                function_name!(),
                deployment_job_id,
                revision_id,
                target_type,
                target_id,
                event_name,
                is_success
            )
        })?;

        // Add successful deployment event if there was one
        if is_success {
            let deployment_id = tx
                .query_one(
                    "
            insert into deployment
                (revision_id, target_type, target_id, event_name)
            values
                ($1, $2, $3, $4)
            returning deployment_id
            ",
                    &[&revision_id, &target_type, &target_id, &event_name],
                )
                .await
                .with_context(|| {
                    format!(
                        "{}: {} {} {}",
                        function_name!(),
                        revision_id,
                        target_id,
                        event_name
                    )
                })?
                .get("deployment_id");

            tx.commit().await?;

            return Ok(Some(deployment_id));
        } else {
            tx.commit().await?;
            return Ok(None);
        }
    }

    /// Find all deployment jobs that occurred after the deployment job for a given event.
    ///
    /// All jobs matching (revision, project, pipeline, target_type, target_id) that happened
    /// after a job ((revision, project, pipeline, target_type, target_id, event_name) that
    /// was successful.
    ///
    /// Panics if no such previous job exists.
    ///
    /// Returns jobs in the order they were created, so index 0 is the newest job
    ///
    /// Returns a vec of (job_id, is_retryable, is_success, started_at, finished_at)
    pub async fn list_deployment_jobs_after_event(
        &self,
        revision_id: &str,
        project: &str,
        deployment_pipeline_id: &str,
        target_type: &str,
        target_id: &str,
        after_event_name: &str,
    ) -> Result<Vec<(String, bool, bool, DateTime<Utc>, DateTime<Utc>)>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let starting_point: Option<DateTime<Utc>> = tx
            .query_opt(
                "
            select finished_at
            from deployment_job
            where
                revision_id = $1 and
                target_type = $2 and
                target_id = $3 and
                event_name = $4 and
                finished_at is not null and
                is_success = true
            order by created_at asc
            limit 1
            ",
                &[&revision_id, &target_type, &target_id, &after_event_name],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: starting point {} {} {} {} {} {}",
                    function_name!(),
                    revision_id,
                    project,
                    deployment_pipeline_id,
                    target_type,
                    target_id,
                    after_event_name
                )
            })?
            .map(|r| r.get("finished_at"));

        if starting_point.is_none() {
            return Ok(vec![]);
        }
        let starting_point = starting_point.unwrap();

        let jobs = tx
            .query(
                "
            select deployment_job_id, is_retryable, is_success, started_at, finished_at
            from deployment_job
            where
                revision_id = $1 and
                target_type = $2 and
                target_id = $3 and
                finished_at is not null and
                created_at >= $4
            order by created_at asc
            ",
                &[&revision_id, &target_type, &target_id, &starting_point],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: idempotency {} {} {} {} {} {}",
                    function_name!(),
                    revision_id,
                    project,
                    deployment_pipeline_id,
                    target_type,
                    target_id,
                    after_event_name
                )
            })?
            .iter()
            .map(|r| {
                (
                    r.get("deployment_job_id"),
                    r.get("is_retryable"),
                    r.get("is_success"),
                    r.get("started_at"),
                    r.get("finished_at"),
                )
            })
            .collect();

        tx.commit().await?;

        Ok(jobs)
    }

    /// Returns the jobs in this (revision, project, pipeline, target_type, target_id, event_name).
    /// Returns jobs in the order they were created, where index 0 is the newest job
    ///
    /// Returns vec of (job_id, is_retryable, is_success)
    pub async fn list_deployment_jobs(
        &self,
        revision_id: &str,
        project: &str,
        deployment_pipeline_id: &str,
        target_type: &str,
        target_id: &str,
        event_name: &str,
    ) -> Result<Vec<(String, bool, bool)>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let jobs: Vec<(String, bool, bool)> = tx
            .query(
                "
            select deployment_job_id, is_retryable, is_success
            from deployment_job
            where
                revision_id = $1 and
                target_type = $2 and
                target_id = $3 and
                event_name = $4 and
                finished_at is not null
            order by created_at desc
            ",
                &[&revision_id, &target_type, &target_id, &event_name],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: idempotency {} {} {} {} {} {}",
                    function_name!(),
                    revision_id,
                    project,
                    deployment_pipeline_id,
                    target_type,
                    target_id,
                    event_name
                )
            })?
            .iter()
            .map(|r| {
                (
                    r.get("deployment_job_id"),
                    r.get("is_retryable"),
                    r.get("is_success"),
                )
            })
            .collect();

        tx.commit().await?;

        Ok(jobs)
    }

    /// Creates a deployment job if it doesn't already exist.
    ///
    /// Returns an existing job if:
    ///  - The job exists and is unfinished for the same tuple (revision, target_type, target_id, event), or,
    ///
    /// This means retries ARE jobs with very similar parameters!
    ///
    /// Returns job_id
    pub async fn create_deployment_job(
        &self,
        revision_id: &str,
        project: &str,
        deployment_pipeline_id: &str,
        target_type: &str,
        target_id: &str,
        event_name: &str,
    ) -> Result<String, anyhow::Error> {
        let mut con = self.db.get().await?;

        let tx = con.transaction().await?;

        let unfinished_job_id: Option<String> = tx
            .query_opt(
                "
            select deployment_job_id
            from deployment_job
            where
                revision_id = $1 and
                target_type = $2 and
                target_id = $3 and
                event_name = $4 and
                finished_at is null
            order by created_at desc
            limit 1
            ", // Get only the latest attempt, since retries kickoff new deployment jobs
                &[&revision_id, &target_type, &target_id, &event_name],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: idempotency {} {} {} {} {} {}",
                    function_name!(),
                    revision_id,
                    project,
                    deployment_pipeline_id,
                    target_type,
                    target_id,
                    event_name
                )
            })?
            .map(|r| r.get("deployment_job_id"));

        if let Some(unfinished_job_id) = unfinished_job_id {
            return Ok(unfinished_job_id);
        }

        let deployment_job_id = tx
            .query_one(
                "
            insert into deployment_job
                (
                    revision_id,
                    project_id,
                    deployment_pipeline_id,
                    target_type,
                    target_id,
                    event_name
                )
            values
                ($1, $2, $3, $4, $5, $6)
            returning deployment_job_id
            ",
                &[
                    &revision_id,
                    &project,
                    &deployment_pipeline_id,
                    &target_type,
                    &target_id,
                    &event_name,
                ],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: creation {} {} {} {} {} {}",
                    function_name!(),
                    revision_id,
                    project,
                    deployment_pipeline_id,
                    target_type,
                    target_id,
                    event_name
                )
            })?
            .get("deployment_job_id");

        tx.commit().await?;

        Ok(deployment_job_id)
    }

    fn row_to_deployment_event(row: &Row) -> Deployment {
        Deployment {
            deployment_id: row.get("deployment_id"),
            revision_id: row.get("revision_id"),
            target_type: row.get("target_type"),
            target_id: row.get("target_id"),
            event_name: row.get("event_name"),
            created_at: row.get("created_at"),
        }
    }

    pub async fn list_deployment_events(&self) -> Result<Vec<Deployment>, anyhow::Error> {
        let deployment_events = self
            .db
            .get()
            .await?
            .query(
                "
            select deployment_id, revision_id, target_type, target_id, event_name, created_at
            from deployment
            ",
                &[],
            )
            .await?
            .iter()
            .map(|row| AntZooStorageClient::row_to_deployment_event(&row))
            .collect();

        Ok(deployment_events)
    }

    pub async fn list_unfinished_deployment_jobs(
        &self,
    ) -> Result<Vec<DeploymentJob>, anyhow::Error> {
        let jobs = self
            .db
            .get()
            .await?
            .query(
                "
            select
                deployment_job_id,
                project_id,
                deployment_pipeline_id,
                revision_id,
                target_type,
                target_id,
                event_name
            from deployment_job
            where
                started_at is null and
                finished_at is null and
                is_success is null
            ",
                &[],
            )
            .await?
            .iter()
            .map(|row| DeploymentJob {
                job_id: row.get("deployment_job_id"),
                project_id: row.get("project_id"),
                deployment_pipeline_id: row.get("deployment_pipeline_id"),
                revision: row.get("revision_id"),
                target_type: row.get("target_type"),
                target_id: row.get("target_id"),
                event_name: row.get("event_name"),
            })
            .collect();

        Ok(jobs)
    }

    /// Returns (deployment_id, revision_id, target_type, target_id, event_name, created_at)
    pub async fn list_deployment_events_in_pipeline_revision(
        &self,
        project: &str,
        revision_id: &str,
    ) -> Result<Vec<Deployment>, anyhow::Error> {
        let deployment_events = self
            .db
            .get()
            .await?
            .query(
                "
            select
                deployment_id,
                deployment.revision_id,
                target_type,
                target_id,
                event_name,
                deployment.created_at
            from deployment
                join revision on revision.revision_id = deployment.revision_id
            where
                deployment.revision_id = $1 and
                revision.project_id = $2
            order by deployment.created_at asc
            ",
                &[&revision_id, &project],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), project, revision_id))?
            .iter()
            .map(|row| AntZooStorageClient::row_to_deployment_event(&row))
            .collect();

        Ok(deployment_events)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub id: String,
    pub seq: i32,
    pub project_id: String,
    pub version: String,
}

impl AntZooStorageClient {
    fn row_to_revision(&self, row: &Row) -> Revision {
        Revision {
            id: row.get("revision_id"),
            seq: row.get("revision_seq"),
            project_id: row.get("project_id"),
            version: row.get("deployment_version"),
        }
    }

    pub async fn list_revisions(&self) -> Result<Vec<Revision>, anyhow::Error> {
        let deployment_events = self
            .db
            .get()
            .await?
            .query(
                "
            select revision_id, project_id, deployment_version, revision_seq
            from revision
            order by revision_seq desc
            ",
                &[],
            )
            .await?
            .iter()
            .map(|row| self.row_to_revision(row))
            .collect();

        Ok(deployment_events)
    }

    pub async fn get_revision(&self, revision_id: &str) -> Result<Revision, anyhow::Error> {
        let row = self
            .db
            .get()
            .await?
            .query_one(
                "
            select revision_id, project_id, deployment_version, revision_seq
            from revision
            where revision_id = $1
            ",
                &[&revision_id],
            )
            .await
            .context(format!("{}: {}", function_name!(), revision_id))?;

        Ok(self.row_to_revision(&row))
    }

    /// Returns revisions that have the given event on that target.
    ///
    /// For example:
    ///  - Used to return the latest revision with 'stage-finished' on a stage, representing the
    ///    last successful deployment to that entire stage.
    ///
    ///  - Used to return the latest revision with 'host-group-started' on a host. If it's the same
    ///    as the one with 'host-group-finished' then there's nothing in progress!
    ///
    /// Returns vec of (Revision, deployment created at)
    pub async fn list_revisions_with_event(
        &self,
        deployment_pipeline_id: &str,
        target_type: &str,
        target_id: &str,
        event_name: &str,
    ) -> Result<Vec<(Revision, DateTime<Utc>)>, anyhow::Error> {
        let con = self.db.get().await?;

        let revisions = con
            .query(
                "
            select deployment.revision_id, r.revision_seq, r.project_id, r.deployment_version, deployment.created_at
            from deployment
                join revision r on r.revision_id = deployment.revision_id
                join deployment_pipeline p on p.project_id = 
                    r.project_id
            where
                deployment_pipeline_id = $1 and
                target_type = $2 and
                target_id = $3 and
                event_name = $4
            order by deployment_seq desc
            ",
                &[
                    &deployment_pipeline_id,
                    &target_type,
                    &target_id,
                    &event_name,
                ],
            )
            .await?
            .iter().map(|row| (self.row_to_revision(&row), row.get("created_at"))).collect();

        Ok(revisions)
    }

    /// For example, finding all revisions without a "pipeline-finished" event would be finding
    /// all IN PROGRESS revisions.
    ///
    /// Returns (Revision, pipeline_id)
    pub async fn list_revisions_missing_event(
        &self,
        event_name: &str,
    ) -> Result<Vec<(Revision, String)>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let rows = tx
            .query(
                "
            select
                r.revision_id,
                r.revision_seq,
                r.project_id,
                r.deployment_version,
                p.deployment_pipeline_id
            from revision r
                join deployment_pipeline p on p.project_id = r.project_id
            where r.revision_id not in (
                select revision_id
                from deployment
                where deployment.event_name = $1
            )
            order by r.revision_seq desc;
            ",
                &[&event_name],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), event_name))?;

        let deployments = rows
            .iter()
            .map(|row| (self.row_to_revision(row), row.get("deployment_pipeline_id")))
            .collect();

        Ok(deployments)
    }

    pub async fn host_in_host_group(
        &self,
        host_group_id: &str,
        host_id: &str,
    ) -> Result<bool, anyhow::Error> {
        let exists = self
            .db
            .get()
            .await?
            .query_opt(
                "select 1
                from host_group_host
                where
                    host_group_id = $1 and
                    host_id = $2",
                &[&host_group_id, &host_id],
            )
            .await?
            .is_some();

        Ok(exists)
    }

    pub async fn host_group_exists_by_id(
        &self,
        host_group_id: &str,
    ) -> Result<bool, anyhow::Error> {
        let exists = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select 1
            from host_group
            where
                host_group_id = $1
            ",
                &[&host_group_id],
            )
            .await?
            .is_some();

        Ok(exists)
    }

    pub async fn get_host_group_by_id(
        &self,
        host_group_id: &str,
    ) -> Result<Option<HostGroup>, anyhow::Error> {
        let host_group_name: Option<String> = self
            .db
            .get()
            .await?
            .query_opt(
                "
                select host_group_name
                from host_group
                where host_group_id = $1",
                &[&host_group_id],
            )
            .await?
            .map(|r| r.get("host_group_name"));

        match host_group_name {
            None => Ok(None),
            Some(host_group_name) => self.get_host_group_by_name(&host_group_name).await,
        }
    }

    pub async fn get_host_group_by_stage_id(
        &self,
        stage_id: &str,
    ) -> Result<Option<HostGroup>, anyhow::Error> {
        let host_group_name: Option<String> = self
            .db
            .get()
            .await?
            .query_opt(
                "
                select host_group_name
                from host_group
                    join deployment_pipeline_stage on
                        host_group.host_group_id
                            = deployment_pipeline_stage.stage_type_deploy_host_group_id
                where deployment_pipeline_stage.deployment_pipeline_stage_id = $1",
                &[&stage_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), stage_id))?
            .map(|r| r.get("host_group_name"));

        match host_group_name {
            None => Ok(None),
            Some(host_group_name) => self.get_host_group_by_name(&host_group_name).await,
        }
    }

    pub async fn get_host_group_by_host(
        &self,
        deployment_pipeline_id: &str,
        host_id: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let host_group_id: Option<String> = self
            .db
            .get()
            .await?
            .query_opt(
                "
                select host_group.host_group_id
                from host
                    join host_group_host on host_group_host.host_id = host.host_id
                    join host_group on host_group.host_group_id = host_group_host.host_group_id
                    join deployment_pipeline_stage on
                        host_group.host_group_id
                            = deployment_pipeline_stage.stage_type_deploy_host_group_id
                    join deployment_pipeline on
                        deployment_pipeline.deployment_pipeline_id
                            = deployment_pipeline_stage.deployment_pipeline_id
                where
                    deployment_pipeline.deployment_pipeline_id = $1 and
                    host.host_id = $2
                ",
                &[&deployment_pipeline_id, &host_id],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: {} {}",
                    function_name!(),
                    deployment_pipeline_id,
                    host_id
                )
            })?
            .map(|r| r.get("host_group_id"));

        Ok(host_group_id)
    }

    pub async fn get_host_group_by_name(
        &self,
        name: &str,
    ) -> Result<Option<HostGroup>, anyhow::Error> {
        let host_group_row = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select
                host_group_id,
                host_group_name,
                environment,
                host_group_description,
                created_at,
                updated_at
            from host_group
            where
                host_group_name = $1
            ",
                &[&name],
            )
            .await
            .with_context(|| format!("{}: host_group {}", function_name!(), name))?;

        if host_group_row.is_none() {
            return Ok(None);
        }
        let host_group_row = host_group_row.unwrap();

        let host_group_id: String = host_group_row.get("host_group_id");

        let host_rows = self
            .db
            .get()
            .await?
            .query(
                "
            select
                host.host_id,
                host.architecture_id,
                host_group_host.created_at
            from host_group_host
                join host on host.host_id = host_group_host.host_id
            where host_group_id = $1
        ",
                &[&host_group_id],
            )
            .await
            .with_context(|| format!("{}: hosts {}", function_name!(), host_group_id))?;

        let hosts = host_rows
            .iter()
            .map(|row| -> Result<HostGroupHost, anyhow::Error> {
                Ok(HostGroupHost {
                    name: row.get("host_id"),
                    arch: HostArchitecture::from_str(row.get("architecture_id"))?,
                    added_at: row.get("created_at"),
                })
            })
            .collect::<Result<Vec<HostGroupHost>, anyhow::Error>>()?;

        Ok(Some(HostGroup {
            id: host_group_row.get("host_group_id"),
            name: host_group_row.get("host_group_name"),
            environment: host_group_row.get("environment"),
            description: host_group_row.get("host_group_description"),
            hosts: hosts,
            created_at: host_group_row.get("created_at"),
            updated_at: host_group_row.get("updated_at"),
        }))
    }

    /// Find a host group (group of hosts + environment) from a starting host and project.
    ///
    /// The same host can be specified for multiple projects, but can only be specified a
    /// single time within a project, so this should be unique!
    pub async fn find_host_group_by_host_and_project(
        &self,
        host_id: &str,
        project: &str,
    ) -> Result<Option<(String, String)>, anyhow::Error> {
        let rows = self
            .db
            .get()
            .await?
            .query(
                "
            select hg.host_group_id, hg.environment
            from host_group hg
                join host_group_host hgh on hgh.host_group_id = hg.host_group_id
                join deployment_pipeline_stage s on s.stage_type_deploy_host_group_id
                        = hg.host_group_id
                join deployment_pipeline p on p.deployment_pipeline_id
                    = s.deployment_pipeline_id
            where
                hgh.host_id = $1 and
                p.project_id = $2
            ",
                &[&host_id, &project],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), host_id, project))?;

        if rows.len() > 1 {
            return Err(anyhow::Error::msg(format!(
                "Hosts are only meant to be included in a project once, but found more than that: {:?}",
                rows
            )));
        }
        let row = rows.first();

        let host_group = row.map(|row| (row.get("host_group_id"), row.get("environment")));

        Ok(host_group)
    }

    pub async fn create_host_group(
        &self,
        host_group_id: &str,
        environment: &str,
    ) -> Result<String, anyhow::Error> {
        let row = self
            .db
            .get()
            .await?
            .query_one(
                "
            insert into host_group
                (host_group_name, environment)
            values
                ($1, $2)
            returning host_group_id
            ",
                &[&host_group_id, &environment],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), host_group_id, environment))?;

        Ok(row.get("host_group_id"))
    }

    pub async fn add_host_to_host_group(
        &self,
        host_group_id: &str,
        host_id: &str,
    ) -> Result<DateTime<Utc>, anyhow::Error> {
        let created_at = self
            .db
            .get()
            .await?
            .query_one(
                "
            insert into host_group_host
                (host_group_id, host_id)
            values
                ($1, $2)
            returning created_at
            ",
                &[&host_group_id, &host_id],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), host_group_id, host_id))?
            .get("created_at");

        Ok(created_at)
    }

    /// Returns true if the host was in the group, false otherwise.
    pub async fn remove_host_from_host_group(
        &self,
        host_group_id: &str,
        host_id: &str,
    ) -> Result<bool, anyhow::Error> {
        let rows = self
            .db
            .get()
            .await?
            .execute(
                "
            delete from host_group_host
            where
                host_group_id = $1 and
                host_id = $2
            ",
                &[&host_group_id, &host_id],
            )
            .await?;

        Ok(rows == 1)
    }

    pub async fn get_host(
        &self,
        host_id: &str,
    ) -> Result<Option<(String, HostArchitecture)>, anyhow::Error> {
        let host = self
            .db
            .get()
            .await?
            .query_opt(
                "
                select host_id, architecture_id
                from host
                where host_id = $1",
                &[&host_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), host_id))?
            .map(|row| -> Result<(String, HostArchitecture), anyhow::Error> {
                Ok((
                    row.get("host_id"),
                    HostArchitecture::from_str(row.get("architecture_id"))?,
                ))
            })
            .transpose()?;

        Ok(host)
    }
}
