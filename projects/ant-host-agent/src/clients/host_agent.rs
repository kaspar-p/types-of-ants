use super::host::Host;
use crate::common::{
    get_project_logs::{GetProjectLogsRequest, GetProjectLogsResponse},
    kill_project::{KillProjectRequest, KillProjectResponse},
    launch_project::LaunchProjectResponse,
};
use ant_metadata::Project;
use anyhow::Result;
use hyper::Method;
use std::{path::Path, time::Duration};

pub struct HostAgentClient<'h> {
    host: &'h Host,
    client: reqwest::Client,
}

impl<'h> HostAgentClient<'h> {
    pub async fn connect(host: &'h Host) -> Result<HostAgentClient> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;

        Ok(HostAgentClient { host, client })
    }

    pub async fn kill_project(&self, project: Project) -> Result<KillProjectResponse> {
        let req = self
            .client
            .request(
                Method::POST,
                format!("http://{}/kill_project", self.host.ip),
            )
            .json(&KillProjectRequest { project })
            .build()?;
        let res = self.client.execute(req).await?;
        let data = res.json::<KillProjectResponse>().await?;
        return Ok(data);
    }

    pub async fn launch_project<P>(
        &self,
        project: Project,
        artifact_path: &P,
    ) -> Result<LaunchProjectResponse>
    where
        P: AsRef<Path>,
    {
        let file = std::fs::read(&artifact_path)?;
        let name = artifact_path
            .as_ref()
            .file_name()
            .ok_or(anyhow::Error::msg("No filename!"))?
            .to_str()
            .ok_or(anyhow::Error::msg("Filename not UTF8!"))?
            .to_owned();

        let file_part = reqwest::multipart::Part::bytes(file)
            .file_name(name)
            .mime_str("image/jpg")
            .unwrap();

        let form = reqwest::multipart::Form::new()
            .part("project", reqwest::multipart::Part::text(project.as_str()))
            .part("artifact", file_part);

        let req = self
            .client
            .request(
                Method::PATCH,
                format!("http://{}/launch_project", self.host.ip),
            )
            .multipart(form)
            .build()?;

        let res = self.client.execute(req).await?;
        let data = res.json::<LaunchProjectResponse>().await?;
        return Ok(data);
    }

    pub async fn get_project_logs(&self) -> Result<GetProjectLogsResponse> {
        let req = self
            .client
            .request(
                Method::POST,
                format!("http://{}/get_project_logs", self.host.ip),
            )
            .json(&GetProjectLogsRequest {})
            .build()?;
        let res = self.client.execute(req).await?;
        let data = res.json::<GetProjectLogsResponse>().await?;
        return Ok(data);
    }
}
