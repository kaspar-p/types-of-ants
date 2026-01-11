use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use ant_library::db::{Database, DatabaseConfig, TypesOfAntsDatabase, database_connection};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use ant_library::host_architecture::HostArchitecture;

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
    pub description: Option<String>,
    pub hosts: Vec<HostGroupHost>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

    pub async fn get_latest_artifacts_for_project_for_all_architectures(
        &self,
        project: &str,
    ) -> Result<Vec<(String, String, DateTime<Utc>)>, anyhow::Error> {
        let con = self.db.get().await?;

        let artifacts = con
            .query(
                "
            select
                project_revision.deployment_version,
                artifact.architecture_id,
                artifact.created_at
            from artifact
                join project_revision on artifact.project_revision_id 
                    = project_revision.project_revision_id
            where project_id = $1
            order by artifact.created_at desc
        ",
                &[&project],
            )
            .await?
            .iter()
            .map(|r| {
                (
                    r.get("deployment_version"),
                    r.get("architecture_id"),
                    r.get("created_at"),
                )
            })
            .collect();

        Ok(artifacts)
    }

    pub async fn get_artifact(
        &self,
        project: &str,
        arch: Option<&HostArchitecture>,
        version: &str,
    ) -> Result<Option<(String, PathBuf)>, anyhow::Error> {
        let con = self.db.get().await?;

        let exists = con
            .query_opt(
                "
            select artifact_id, local_path
            from artifact
                join project_revision on project_revision.project_revision_id
                    = artifact.project_revision_id
            where
                project_revision.project_id = $1 and
                artifact.architecture_id = $2 and
                project_revision.deployment_version = $3
            ",
                &[&project, &arch.map(|a| a.as_str()), &version],
            )
            .await?
            .map(|row| -> Result<(String, PathBuf), anyhow::Error> {
                Ok((
                    row.get("artifact_id"),
                    PathBuf::from_str(row.get("local_path"))?,
                ))
            })
            .transpose()?;

        Ok(exists)
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
            select project_revision_id
            from project_revision
            where
                project_id = $1 and
                deployment_version = $2",
                &[&project, &version],
            )
            .await?
            .get("project_revision_id");

        let rows = tx
            .query(
                "
        select architecture.architecture_id
        from architecture
        where architecture.architecture_id not in (
            select artifact.architecture_id
            from artifact
            where artifact.project_revision_id = $1
        )
        ",
                &[&revision],
            )
            .await?;

        let architectures = rows
            .iter()
            .map(|row| HostArchitecture::from_str(row.get("architecture_id")))
            .collect::<Result<Vec<HostArchitecture>, anyhow::Error>>()?;

        Ok(architectures)
    }

    pub async fn register_artifact(
        &self,
        project: &str,
        arch: Option<&HostArchitecture>,
        version: &str,
        path: &Path,
    ) -> Result<String, anyhow::Error> {
        let mut con = self.db.get().await?;

        let tx = con.transaction().await?;

        let revision_id: Option<String> = tx
            .query_opt(
                "
                select project_revision_id
                from project_revision
                where
                    project_id = $1 and
                    deployment_version = $2
            ",
                &[&project, &version],
            )
            .await?
            .map(|r| r.get("project_revision_id"));

        let revision_id = match revision_id {
            None => {
                let project_revision_id: String = tx
                    .query_one(
                        "
                insert into project_revision
                    (project_id, deployment_version)
                values
                    ($1, $2)
                returning project_revision_id
                ",
                        &[&project, &version],
                    )
                    .await?
                    .get("project_revision_id");

                project_revision_id
            }
            Some(id) => id,
        };

        let artifact_id = tx
            .query_one(
                "
            insert into artifact
                (project_revision_id, architecture_id, local_path)
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

    pub async fn deployment_pipeline_exists_by_project(
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
            .await?;

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

    pub async fn get_deployment_pipeline_stage_by_name(
        &self,
        deployment_pipeline_id: &str,
        stage_name: &str,
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
                deployment_pipeline_id = $1 and
                stage_name = $2
        ",
                &[&deployment_pipeline_id, &stage_name],
            )
            .await?
            .map(|row| row.get("deployment_pipeline_stage_id"));

        Ok(stage_id)
    }

    pub async fn get_deployment_pipeline_build_stage(
        &self,
        deployment_pipeline_id: &str,
    ) -> Result<String, anyhow::Error> {
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
                project_id = $1 and
                stage_type = 'build'
        ",
                &[&deployment_pipeline_id],
            )
            .await?
            .map(|r| r.get("deployment_pipeline_stage_id"))
            .ok_or_else(|| {
                anyhow::Error::msg(format!(
                    "{}{}",
                    "Could not find a build stage for pipeline",
                    format!("{deployment_pipeline_id}, but all pipelines should have one!")
                ))
            })?;

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
            .await?;

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
            .await?
            .get("deployment_pipeline_stage_id");

        Ok(stage_id)
    }

    pub async fn get_hosts_in_stage(&self, stage_id: &str) -> Result<Vec<String>, anyhow::Error> {
        let rows = self
            .db
            .get()
            .await?
            .query(
                "
            select
                host.host_id
            from host
                join host_group_host on host_group_host.host_id = host.host_id
                join deployment_pipeline_stage on 
                    deployment_pipeline_stage.stage_type_deploy_host_group_id
                        = host_group_host.host_group_id
            where
                deployment_pipeline_stage_id = $1
        ",
                &[&stage_id],
            )
            .await?;

        let hosts = rows
            .iter()
            .map(|row| row.get("host_id"))
            .collect::<Vec<String>>();

        Ok(hosts)
    }

    pub async fn get_deployment_history_on_host(
        &self,
        host_id: &str,
    ) -> Result<Vec<(String, String, DateTime<Utc>)>, anyhow::Error> {
        let rows = self
            .db
            .get()
            .await?
            .query(
                "
            select
                deployment.deployment_id,
                project_revision.deployment_version,
                project_revision.created_at
            from deployment
                join project_revision on deployment.project_revision_id
                    = project_revision.project_revision_id
                join deployment_pipeline_stage on deployment.deployment_pipeline_stage_id
                    = deployment_pipeline_stage.deployment_pipeline_stage_id
            where
                deployment_pipeline_stage.stage_type_deploy_host_group_id =
                    (select host_group_id from host_group_host where host_id = $1)
            order by created_at desc
        ",
                &[&host_id],
            )
            .await?;

        let revisions = rows
            .iter()
            .map(|row| {
                (
                    row.get("deployment_id"),
                    row.get("deployment_version"),
                    row.get("created_at"),
                )
            })
            .collect::<Vec<(String, String, DateTime<Utc>)>>();

        Ok(revisions)
    }

    pub async fn make_deployment(
        &self,
        stage_id: &str,
        project: &str,
        version: &str,
    ) -> Result<String, anyhow::Error> {
        let mut con = self.db.get().await?;

        let tx = con.transaction().await?;

        let revision: String = tx
            .query_one(
                "
            select project_revision_id
            from project_revision
            where
                project_id = $1 and
                deployment_version = $2",
                &[&project, &version],
            )
            .await?
            .get("project_revision_id");

        let deployment_id = tx
            .query_one(
                "
        insert into deployment
            (deployment_pipeline_stage_id, project_revision_id)
        values
            ($1, $2)
        returning deployment_id
        ",
                &[&stage_id, &revision],
            )
            .await?
            .get("deployment_id");

        tx.commit().await?;

        Ok(deployment_id)
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
                host_group_description,
                created_at,
                updated_at
            from host_group
            where
                host_group_name = $1
            ",
                &[&name],
            )
            .await?;

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
            .await?;

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
            .await?;

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
            .await?;

        Ok(row.get("host_group_id"))
    }

    pub async fn add_host_to_host_group(
        &self,
        host_group_id: &str,
        host_id: &str,
    ) -> Result<(), anyhow::Error> {
        self.db
            .get()
            .await?
            .execute(
                "
            insert into host_group_host
                (host_group_id, host_id)
            values
                ($1, $2)
            ",
                &[&host_group_id, &host_id],
            )
            .await?;

        Ok(())
    }

    pub async fn remove_host_from_host_group(
        &self,
        host_group_id: &str,
        host_id: &str,
    ) -> Result<(), anyhow::Error> {
        self.db
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

        Ok(())
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
            .await?
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
