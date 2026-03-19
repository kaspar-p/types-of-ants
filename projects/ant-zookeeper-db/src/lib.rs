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
    pub project: String,
    pub environment: String,
    pub description: Option<String>,
    pub hosts: Vec<HostGroupHost>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub deployment_id: String,
    pub revision_id: String,
    pub event: String,
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

    /// When building a new project, the project associates itself with some revision, some VERSION.
    ///
    /// This is allowed if the revision is still "collecting", and has not progressed beyond the build
    /// phase of the pipeline, but not allowed if we've started deploying this artifact.
    ///
    /// This is the inverse of the Version Set, which forces itself as one-final-thing onto the pipeline
    /// to be used for its artifacts. Here we need to "collect"/"poll" until we decide to release it further
    /// into the pipeline, closing that collection phase.
    ///
    /// Returns (revision_id, is_new)
    pub async fn upsert_revision(&self, version: &str) -> Result<(String, bool), anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let revision_id: Option<String> = tx
            .query_opt(
                "
                select revision_id
                from revision
                where
                    deployment_version = $1
            ",
                &[&version],
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
                        (deployment_version)
                    values
                        ($1)
                    returning revision_id
                    ",
                        &[&version],
                    )
                    .await?
                    .get("revision_id");

                tx.commit().await?;

                return Ok((revision_id, true));
            }
        }
    }

    /// Returns (artifact id, version, local path)
    pub async fn get_artifact_by_revision(
        &self,
        revision_id: &str,
        project: &str,
        arch: Option<&HostArchitecture>,
    ) -> Result<Option<(String, String, PathBuf)>, anyhow::Error> {
        let con = self.db.get().await?;

        let exists = con
            .query_opt(
                "
            select artifact.artifact_id, revision.deployment_version, artifact.local_path
            from artifact
                join revision on artifact.revision_id = revision.revision_id
            where
                artifact.revision_id = $1 and
                artifact.project_id = $2 and
                artifact.architecture_id = $3
            ",
                &[&revision_id, &project, &arch.map(|a| a.as_str())],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: {} {} {:?}",
                    function_name!(),
                    revision_id,
                    project,
                    arch
                )
            })?
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
        project: &str,
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
                where
                    artifact.revision_id = $1 and
                    artifact.project_id = $2
            )
            ",
                &[&revision_id, &project],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), project, revision_id))?;

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
                deployment_version = $1",
                &[&version],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), project, version))?
            .get("revision_id");

        return Ok(self
            .missing_artifacts_for_revision_id(&project, &revision)
            .await?);
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
        project: &str,
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
                (project_id, revision_id, architecture_id, local_path)
            values
                ($1, $2, $3, $4)
            returning artifact_id
            ",
                &[
                    &project,
                    &revision_id,
                    &arch.map(|a| a.as_str()),
                    &path
                        .as_os_str()
                        .to_str()
                        .expect(&format!("bad artifact path: {}", path.display())),
                ],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: {} {:?} {}",
                    function_name!(),
                    revision_id,
                    arch,
                    path.display()
                )
            })?
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

    pub async fn create_deployment_pipeline(&self, name: &str) -> Result<String, anyhow::Error> {
        let exists = self
            .db
            .get()
            .await?
            .query_one(
                "
            insert into deployment_pipeline
                (deployment_pipeline_name)
            values
                ($1)
            returning deployment_pipeline_id
            ",
                &[&name],
            )
            .await?
            .get("deployment_pipeline_id");

        Ok(exists)
    }

    /// Returns vec of (pipeline_id, pipeline_name)
    pub async fn list_deployment_pipelines(&self) -> Result<Vec<(String, String)>, anyhow::Error> {
        let pipelines = self
            .db
            .get()
            .await?
            .query(
                "
            select deployment_pipeline_id, deployment_pipeline_name
            from deployment_pipeline
            ",
                &[],
            )
            .await?
            .into_iter()
            .map(|row| {
                (
                    row.get("deployment_pipeline_id"),
                    row.get("deployment_pipeline_name"),
                )
            })
            .collect();

        Ok(pipelines)
    }

    pub async fn get_deployment_pipeline_by_name(
        &self,
        name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let exists = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select deployment_pipeline_id
            from deployment_pipeline
            where deployment_pipeline_name = $1
            ",
                &[&name],
            )
            .await?
            .map(|r| r.get("deployment_pipeline_id"));

        Ok(exists)
    }

    #[deprecated]
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

    /// Returns vec of (stage_name, stage_id, stage_type, stage_type_build_project_id) UNORDERED
    pub async fn list_deployment_pipeline_stages(
        &self,
        deployment_pipeline_id: &str,
    ) -> Result<Vec<(String, String, String, Option<String>)>, anyhow::Error> {
        let rows = self
            .db
            .get()
            .await?
            .query(
                "
            select
                stage_name,
                deployment_pipeline_stage_id,
                stage_type,
                stage_type_build_project_id
            from deployment_pipeline_stage
            where
                deployment_pipeline_id = $1
        ",
                &[&deployment_pipeline_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), deployment_pipeline_id))?;

        let stages = rows
            .iter()
            .map(|row| {
                (
                    row.get("stage_name"),
                    row.get("deployment_pipeline_stage_id"),
                    row.get("stage_type"),
                    row.get("stage_type_build_project_id"),
                )
            })
            .collect();

        Ok(stages)
    }

    pub async fn list_deployment_stages_with_no_previous_adjacencies(
        &self,
        deployment_pipeline_id: &str,
    ) -> Result<Vec<String>, anyhow::Error> {
        let stage_ids = self
            .db
            .get()
            .await?
            .query(
                "
            select deployment_pipeline_stage_id
            from deployment_pipeline_stage
            where 
                deployment_pipeline_id = $1 and
                deployment_pipeline_stage_id not in (
                    select to_stage_id as deployment_pipeline_stage_id
                    from deployment_pipeline_stage_edge
                )
            order by stage_name
            ",
                &[&deployment_pipeline_id],
            )
            .await?
            .into_iter()
            .map(|row| row.get("deployment_pipeline_stage_id"))
            .collect();

        Ok(stage_ids)
    }

    /// Gets all the stages `s` that have a direct edge like (prev, s).
    ///
    /// If None then finds all stages with no adjacency
    pub async fn list_deployment_pipeline_stages_after(
        &self,
        previous_stage_id: &str,
    ) -> Result<Vec<String>, anyhow::Error> {
        let after = self
            .db
            .get()
            .await?
            .query(
                "
            select to_stage_id
            from deployment_pipeline_stage_edge
            where
                from_stage_id = $1
            order by created_at
            ",
                &[&previous_stage_id],
            )
            .await?
            .into_iter()
            .map(|row| row.get("to_stage_id"))
            .collect();

        Ok(after)
    }

    /// Returns (pipeline_id, stage_name, stage_type, stage_type_build_project_id)
    pub async fn get_deployment_pipeline_stage(
        &self,
        stage_id: &str,
    ) -> Result<Option<(String, String, String, Option<String>)>, anyhow::Error> {
        let stage = self
            .db
            .get()
            .await?
            .query_opt(
                "
            select deployment_pipeline_id, stage_name, stage_type, stage_type_build_project_id
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
                    row.get("stage_type"),
                    row.get("stage_type_build_project_id"),
                )
            });

        Ok(stage)
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
            select deployment_pipeline_stage.deployment_pipeline_stage_id
            from deployment_pipeline_stage
                join deployment_pipeline_stage_host_group on
                    deployment_pipeline_stage_host_group.deployment_pipeline_stage_id
                        = deployment_pipeline_stage.deployment_pipeline_stage_id
            where
                host_group_id = $1
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
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let deleted = tx
            .execute(
                "
            delete from deployment_pipeline_stage
            where deployment_pipeline_stage_id = $1
            ",
                &[&stage_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), stage_id))?;

        // Delete edges from adjacency list
        tx.execute(
            "
        delete from deployment_pipeline_stage_edge
        where
            from_stage_id = $1 or to_stage_id = $1
        ",
            &[&stage_id],
        )
        .await
        .with_context(|| format!("{}: edges {}", function_name!(), stage_id))?;

        // Delete host group associations
        tx.execute(
            "
        delete from deployment_pipeline_stage_host_group
        where deployment_pipeline_stage_id = $1
        ",
            &[],
        )
        .await
        .with_context(|| format!("{}: host groups {}", function_name!(), stage_id))?;

        if deleted != 1 {
            panic!(
                "Deleted {deleted} stages '{stage_id}' but meant to just delete a single stage!"
            );
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn create_deployment_pipeline_deployment_stage(
        &self,
        deployment_pipeline_id: &str,
        stage_name: &str,
        stage_type: &str,
        deploy_stage_host_group_ids: Option<&Vec<String>>,
        build_stage_project_id: Option<&str>,
        previous_stage_ids: &Vec<String>,
    ) -> Result<String, anyhow::Error> {
        let mut con = self.db.get().await?;

        let tx = con.transaction().await?;

        let stage_id: String = tx
            .query_one(
                "
            insert into deployment_pipeline_stage
            (
                deployment_pipeline_id,
                stage_name,
                stage_type,
                stage_type_build_project_id
            )
            values
                ($1, $2, $3, $4)
            returning deployment_pipeline_stage_id
        ",
                &[
                    &deployment_pipeline_id,
                    &stage_name,
                    &stage_type,
                    &build_stage_project_id,
                ],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: {} {} {} {:?} {}",
                    function_name!(),
                    deployment_pipeline_id,
                    stage_name,
                    deploy_stage_host_group_ids.unwrap_or(&vec![]).join(", "),
                    build_stage_project_id,
                    previous_stage_ids.join(", ")
                )
            })?
            .get("deployment_pipeline_stage_id");

        // previous edges
        for prev_stage in previous_stage_ids {
            tx.execute(
                "
            insert into deployment_pipeline_stage_edge
                (from_stage_id, to_stage_id)
            values
                ($1, $2)
            ",
                &[&prev_stage, &stage_id],
            )
            .await
            .with_context(|| format!("{}: edge {} {}", function_name!(), prev_stage, stage_id))?;
        }

        // host group 1:many membership
        for host_group_id in deploy_stage_host_group_ids.unwrap_or(&vec![]) {
            tx.execute(
                "
            insert into deployment_pipeline_stage_host_group
                (deployment_pipeline_stage_id, host_group_id)
            values
                ($1, $2)
            ",
                &[&stage_id, &host_group_id],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: host group {} {}",
                    function_name!(),
                    stage_id,
                    host_group_id
                )
            })?;
        }

        tx.commit().await?;

        Ok(stage_id)
    }

    pub async fn get_deployment(
        &self,
        revision_id: &str,
        event: &str,
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
                deployment_event = $2
            ",
                &[&revision_id, &event],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), revision_id, &event))?
            .map(|row| row.get("deployment_id"));

        Ok(deployment_id)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentJob {
    pub job_id: String,
    pub revision: String,
    pub event_document: String,
}

impl AntZooStorageClient {
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

    /// Returns (created_at, is_successful, is_retryable, updated_at, started_at, finished_at)
    pub async fn get_deployment_job(
        &self,
        deployment_job_id: &str,
    ) -> Result<
        Option<(
            DateTime<Utc>,
            bool,
            bool,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        )>,
        anyhow::Error,
    > {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let job = tx
            .query_opt(
                "
            select created_at, updated_at, is_success, is_retryable, started_at, finished_at
            from deployment_job
            where
                deployment_job_id = $1
            ",
                &[&deployment_job_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), deployment_job_id))?
            .map(|row| {
                (
                    row.get("created_at"),
                    row.get("is_success"),
                    row.get("is_retryable"),
                    row.get("updated_at"),
                    row.get("started_at"),
                    row.get("finished_at"),
                )
            });

        tx.commit().await?;

        Ok(job)
    }

    pub async fn set_deployment_job_retryable(
        &self,
        deployment_job_id: &str,
        retryable: bool,
    ) -> Result<DateTime<Utc>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        // Mark deployment job is_retryable
        let updated_at = tx
            .query_one(
                "
            update deployment_job
            set
                is_retryable = $1,
                updated_at = now()
            where
                deployment_job_id = $2
            returning updated_at
            ",
                &[&retryable, &deployment_job_id],
            )
            .await
            .with_context(|| format!("{}: {} {}", function_name!(), deployment_job_id, retryable))?
            .get("updated_at");

        tx.commit().await?;

        Ok(updated_at)
    }

    /// Returns Some(created Deployment ID) if the job was successful, None if it wasn't,
    /// since there was no deployment resulting of it.
    pub async fn complete_deployment_job(
        &self,
        deployment_job_id: &str,
        revision_id: &str,
        event: &str,
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
                "{}: mark {} {} {} {}",
                function_name!(),
                deployment_job_id,
                revision_id,
                event,
                is_success
            )
        })?;

        // Add successful deployment event if there was one
        if is_success {
            let deployment_id = tx
                .query_one(
                    "
            insert into deployment
                (revision_id, deployment_event)
            values
                ($1, $2)
            returning deployment_id
            ",
                    &[&revision_id, &event],
                )
                .await
                .with_context(|| format!("{}: {} {}", function_name!(), revision_id, event,))?
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
    /// All jobs matching (revision) that happened
    /// after a job (revision, event) that
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
        after_event: &str,
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
                deployment_event = $2 and
                finished_at is not null and
                is_success = true
            order by created_at asc
            limit 1
            ",
                &[&revision_id, &after_event],
            )
            .await
            .with_context(|| {
                format!(
                    "{}: starting point {} {}",
                    function_name!(),
                    revision_id,
                    after_event
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
                finished_at is not null and
                created_at >= $2
            order by created_at asc
            ",
                &[&revision_id, &starting_point],
            )
            .await
            .with_context(|| format!("{}: r {} {}", function_name!(), revision_id, after_event))?
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

    /// Returns the jobs in the same revision created for this deployment event.
    /// Returns jobs in the order they were created, where index 0 is the newest job
    ///
    /// Returns vec of (job_id, is_retryable, is_success)
    pub async fn list_deployment_jobs(
        &self,
        revision_id: &str,
        event: &str,
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
                deployment_event = $2 and
                finished_at is not null
            order by created_at desc
            ",
                &[&revision_id, &event],
            )
            .await
            .with_context(|| format!("{}: idem {} {}", function_name!(), revision_id, event))?
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
    /// If there's already a job for this deployment event that's UNFINISHED, returns that instead.
    ///
    /// This means retries ARE jobs with very similar parameters!
    ///
    /// Returns (job_id, is_new)
    pub async fn create_deployment_job_idempotently(
        &self,
        revision_id: &str,
        event: &str,
    ) -> Result<(String, bool), anyhow::Error> {
        let mut con = self.db.get().await?;

        let tx = con.transaction().await?;

        let unfinished_job_id: Option<String> = tx
            .query_opt(
                "
            select deployment_job_id
            from deployment_job
            where
                revision_id = $1 and
                deployment_event = $2 and
                finished_at is null
            order by created_at desc
            limit 1
            ", // Get only the latest attempt, since retries kickoff new deployment jobs
                &[&revision_id, &event],
            )
            .await
            .with_context(|| format!("{}: idem {} {}", function_name!(), revision_id, event))?
            .map(|r| r.get("deployment_job_id"));

        if let Some(unfinished_job_id) = unfinished_job_id {
            return Ok((unfinished_job_id, false));
        }

        let deployment_job_id = tx
            .query_one(
                "
            insert into deployment_job
                (revision_id, deployment_event)
            values
                ($1, $2)
            returning deployment_job_id
            ",
                &[&revision_id, &event],
            )
            .await
            .with_context(|| format!("{}: creation {} {}", function_name!(), revision_id, event))?
            .get("deployment_job_id");

        tx.commit().await?;

        Ok((deployment_job_id, true))
    }

    fn row_to_deployment_event(row: &Row) -> Deployment {
        Deployment {
            deployment_id: row.get("deployment_id"),
            revision_id: row.get("revision_id"),
            event: row.get("deployment_event"),
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
            select deployment_id, revision_id, deployment_event, created_at
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
                revision_id,
                deployment_event
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
                revision: row.get("revision_id"),
                event_document: row.get("deployment_event"),
            })
            .collect();

        Ok(jobs)
    }

    pub async fn list_deployment_events_in_revision(
        &self,
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
                deployment_event,
                deployment.created_at
            from deployment
                join revision on revision.revision_id = deployment.revision_id
            where
                deployment.revision_id = $1
            order by deployment.created_at asc
            ",
                &[&revision_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), revision_id))?
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
    pub version: String,
}

impl AntZooStorageClient {
    fn row_to_revision(&self, row: &Row) -> Revision {
        Revision {
            id: row.get("revision_id"),
            seq: row.get("revision_seq"),
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
            select revision_id, deployment_version, revision_seq
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
            select revision_id, deployment_version, revision_seq
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
        event: &str,
    ) -> Result<Vec<(Revision, DateTime<Utc>)>, anyhow::Error> {
        let con = self.db.get().await?;

        let revisions = con
            .query(
                "
            select
                deployment.revision_id,
                r.revision_seq,
                r.deployment_version,
                deployment.created_at
            from deployment
                join revision r on r.revision_id = deployment.revision_id
            where
                deployment_event = $1
            order by deployment_seq desc
            ",
                &[&event],
            )
            .await?
            .iter()
            .map(|row| (self.row_to_revision(&row), row.get("created_at")))
            .collect();

        Ok(revisions)
    }

    /// For example, finding all revisions without a "pipeline-finished" event would be finding
    /// all IN PROGRESS revisions.
    ///
    /// Returns (Revision)
    pub async fn list_revisions_missing_event(
        &self,
        event: &str,
    ) -> Result<Vec<Revision>, anyhow::Error> {
        let mut con = self.db.get().await?;
        let tx = con.transaction().await?;

        let rows = tx
            .query(
                "
            select
                r.revision_id,
                r.revision_seq,
                r.deployment_version
            from revision r
            where r.revision_id not in (
                select revision_id
                from deployment
                where
                   deployment.deployment_event = $1
            )
            order by r.revision_seq desc;
            ",
                &[&event],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), event))?;

        let deployments = rows.iter().map(|row| self.row_to_revision(row)).collect();

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

    pub async fn get_host_groups_by_stage_id(
        &self,
        stage_id: &str,
    ) -> Result<Vec<HostGroup>, anyhow::Error> {
        let host_group_names: Vec<String> = self
            .db
            .get()
            .await?
            .query(
                "
                select host_group_name
                from host_group
                    join deployment_pipeline_stage_host_group on
                        deployment_pipeline_stage_host_group.host_group_id
                            = host_group.host_group_id
                where
                    deployment_pipeline_stage_host_group.deployment_pipeline_stage_id = $1",
                &[&stage_id],
            )
            .await
            .with_context(|| format!("{}: {}", function_name!(), stage_id))?
            .iter()
            .map(|r| r.get("host_group_name"))
            .collect();

        let mut host_groups = vec![];
        for host_group_name in host_group_names {
            host_groups.push(
                self.get_host_group_by_name(&host_group_name)
                    .await?
                    .unwrap(),
            );
        }

        return Ok(host_groups);
    }

    #[deprecated]
    pub async fn get_host_group_by_host_in_pipeline(
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
                    join deployment_pipeline_stage_host_group on
                        host_group.host_group_id
                            = deployment_pipeline_stage_host_group.host_group_id
                    join deployment_pipeline_stage on
                        deployment_pipeline_stage_host_group.deployment_pipeline_stage_id
                            = deployment_pipeline_stage.deployment_pipeline_stage_id
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
                project_id,
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
            project: host_group_row.get("project_id"),
            environment: host_group_row.get("environment"),
            description: host_group_row.get("host_group_description"),
            hosts: hosts,
            created_at: host_group_row.get("created_at"),
            updated_at: host_group_row.get("updated_at"),
        }))
    }

    pub async fn create_host_group(
        &self,
        host_group_name: &str,
        project: &str,
        environment: &str,
    ) -> Result<String, anyhow::Error> {
        let row = self
            .db
            .get()
            .await?
            .query_one(
                "
            insert into host_group
                (host_group_name, project_id, environment)
            values
                ($1, $2, $3)
            returning host_group_id
            ",
                &[&host_group_name, &project, &environment],
            )
            .await
            .with_context(|| {
                format!("{}: {} {}", function_name!(), host_group_name, environment)
            })?;

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
