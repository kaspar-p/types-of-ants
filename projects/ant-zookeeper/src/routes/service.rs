use std::collections::HashSet;
use std::fs::exists;
use std::io::Read;
use std::path::PathBuf;
use std::{fs::File, io::Write};

use ant_library::headers::{
    XAntArchitectureHeader, XAntProjectHeader, XAntRevisionHeader, XAntVersionHeader,
};
use anthill_manifest::AnthillManifest;
use axum::debug_handler;
use ant_library::routes::Routes;
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
    routing::post,
    Json,
};
use axum_extra::TypedHeader;
use flate2::read::GzDecoder;
use http::StatusCode;
use humansize::DECIMAL;
use serde::{Deserialize, Serialize};
use tar::Archive;
use tempfile::tempdir_in;
use tokio::fs::create_dir_all;
use tracing::{info, warn};

use crate::event_loop::transition::{is_deployment_complete, DeploymentEvent, Event};
use crate::fs::{
    artifact_file_name, artifact_persist_dir, envs_persist_dir, project_envs_file_name,
    secret_file_path,
};
use crate::{err::AntZookeeperError, state::AntZookeeperState};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectKind {
    Docker,
    Binary {
        /// Defaults to the project's name
        binary_name_override: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct RegisterServiceRequest {
    pub project: String,
    pub kind: ProjectKind,
    pub is_owned: bool,
}

/// Register a new service if not already done
async fn register_service(
    State(state): State<AntZookeeperState>,
    Json(req): Json<RegisterServiceRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if state.db.get_project(&req.project).await? {
        return Ok((
            StatusCode::BAD_REQUEST,
            format!("Project {} already exists", req.project),
        ));
    }

    state
        .db
        .register_project(&req.project, req.is_owned)
        .await?;

    Ok((StatusCode::OK, "Project registered.".to_string()))
}

pub fn binary_systemd_unit(
    project: &str,
    project_description: &str,
    install_dir: &PathBuf,
) -> String {
    let install_dir = install_dir.to_str().unwrap();
    format!(
        "[Unit]
Description={project_description}

[Service]
Type=simple
EnvironmentFile={install_dir}/.env
Environment=TYPESOFANTS_SECRET_DIR={install_dir}/secrets
ExecStart={install_dir}/{project}
WorkingDirectory={install_dir}
Restart=always

[Install]
WantedBy=multi-user.target
"
    )
}

pub fn docker_systemd_unit(
    project: &str,
    project_description: &str,
    install_dir: &PathBuf,
) -> String {
    let install_dir = install_dir.to_str().unwrap();

    format!("[Unit]
Description={project_description}

[Service]
Type=simple
ExecStart=/snap/bin/docker-compose --project-directory={install_dir} up --no-build --force-recreate {project}
ExecStop=/snap/bin/docker-compose --project-directory={install_dir} stop {project}
EnvironmentFile={install_dir}/.env
WorkingDirectory={install_dir}
Restart=always

[Install]
WantedBy=multi-user.target
")
}

/// An API to ingest a new build artifact, a new built version of a service for a given platform.
#[debug_handler]
async fn register_artifact(
    State(state): State<AntZookeeperState>,
    TypedHeader(revision): TypedHeader<XAntRevisionHeader>,
    TypedHeader(project): TypedHeader<XAntProjectHeader>,
    TypedHeader(arch): TypedHeader<XAntArchitectureHeader>,
    TypedHeader(version): TypedHeader<XAntVersionHeader>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AntZookeeperError> {
    let project_id: &String = project.0.as_ref().ok_or(AntZookeeperError::validation_msg(
        "The X-Ant-Project header must be specified.",
    ))?;

    // VALIDATIONS
    {
        if !state.db.get_project(&project_id).await? {
            info!("Registering project [{}] for the first time...", project_id);
            let is_owned = true; // Building our own images means an owned project!
            state.db.register_project(&project_id, is_owned).await?;
        }

        let revision = state.db.get_revision(&revision.0).await?;
        match revision {
            None => {
                return Err(AntZookeeperError::ResourceNotFound(
                    "No such revision.".to_string(),
                ));
            }
            Some(revision) => {
                if revision.activated_at.is_some() {
                    return Err(AntZookeeperError::validation_msg(
                        "Revision has already been activated and is immutable, so cannot be updated.",
                    ));
                }
            }
        }
    }

    let global_tmp_dir = state.root_dir.join("tmp");
    create_dir_all(&global_tmp_dir).await?;

    let dir = artifact_persist_dir(&state.root_dir);
    create_dir_all(&dir).await?;

    let temp_dir = tempdir_in(&global_tmp_dir)?;
    let temp_file_path = temp_dir.path().join("input.tar.gz");
    let mut temp_file = File::create_new(&temp_file_path)?;
    {
        let mut field = multipart
            .next_field()
            .await
            .map_err(|e| {
                warn!("No field in multipart: {e}");
                AntZookeeperError::validation_msg("No field found in multipart request!")
            })?
            .ok_or(AntZookeeperError::validation_msg(
                "No bytes field found in request!",
            ))?;

        while let Some(bytes) = field.chunk().await.unwrap() {
            info!(
                "Wrote [{}] to [{}]...",
                humansize::format_size(bytes.len(), DECIMAL),
                temp_file_path.display()
            );
            temp_file.write_all(&bytes)?;
        }
        temp_file.flush()?;
        info!("Finished writing to [{}]...", temp_file_path.display());
    }

    info!(
        "Validating file contents of [{}]...",
        temp_file_path.display()
    );

    {
        let temp_file = File::open(&temp_file_path)?;
        let gz = GzDecoder::new(&temp_file);
        let mut archive = Archive::new(gz);

        let mut anthill_file_found = false;
        // let mut service_file_found = false;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path.is_dir() {
                continue;
            }

            let file_name = path
                .file_name()
                .expect(format!("entry {} has file name", path.display()).as_str())
                .to_owned();

            info!("Unpacked {}", file_name.display());

            if file_name == ".env" {
                return Err(AntZookeeperError::validation_msg(
                    "Deployment tarball cannot contain any files named '.env'.",
                ));
            }

            if file_name == "anthill.json" {
                anthill_file_found = true;

                let mut manifest_buf = String::new();
                entry.read_to_string(&mut manifest_buf)?;

                let manifest =
                    serde_json::from_str::<AnthillManifest>(&manifest_buf).map_err(|e| {
                        warn!(
                            "Failed to read manifest: {}, {e}",
                            temp_file_path.join(&file_name).display()
                        );
                        AntZookeeperError::validation_msg(
                            "Project configuration file 'anthill.json' was malformed.",
                        )
                    })?;

                info!("Read manifest: {:?}", manifest);

                for secret in manifest.secrets {
                    // TODO better way to get environments
                    for environment in ["beta", "prod"] {
                        if !exists(secret_file_path(&state.root_dir, environment, &secret))? {
                            return Err(AntZookeeperError::validation_msg(
                                format!("Secret '{}' is not present in '{}'.", secret, environment)
                                    .as_str(),
                            ));
                        }
                    }
                }
            }

            // if file_name == format!("{}.service", &project_id).as_str() {
            //     service_file_found = true;
            // }
        }

        if !anthill_file_found {
            return Err(AntZookeeperError::validation_msg(
                "Project configuration file 'anthill.json' must be included in deployment tarball.",
            ));
        }

        // if !service_file_found {
        //     return Err(AntZookeeperError::validation_msg(
        //         format!(
        //             "Service file '{}.service' must be included in deployment tarball.",
        //             &project_id
        //         )
        //         .as_str(),
        //     ));
        // }
    }

    // Write the file to final location
    let filepath = dir.join(artifact_file_name(&project_id, arch.0.as_ref(), &version.0));
    {
        info!("Writing tarball to [{}]", filepath.display());
        std::fs::copy(&temp_file_path, &filepath)?;
    }

    // let (revision_id, is_new) = state.db.upsert_revision(&version.0).await?;

    // {
    //     info!("Scheduling pipelines that build this project to start.");
    //     let all_pipelines = state.db.list_deployment_pipelines().await?;
    //     let pipelines_building_project = {
    //         let mut pipelines_building_project: HashSet<&String> = HashSet::new();
    //         for (pipeline_id, _) in &all_pipelines {
    //             let build_stages = state
    //                 .db
    //                 .list_deployment_stages_with_no_previous_adjacencies(&pipeline_id)
    //                 .await?;
    //             for stage_id in build_stages {
    //                 let stage = state
    //                     .db
    //                     .get_deployment_pipeline_stage(&stage_id)
    //                     .await?
    //                     .unwrap();
    //                 let pipeline_build_project = stage.3.expect(&format!(
    //                     "build stage {stage_id} should have project attached."
    //                 ));

    //                 if pipeline_build_project == *project.0 {
    //                     pipelines_building_project.insert(pipeline_id);
    //                 }
    //             }
    //         }
    //         pipelines_building_project
    //     };

    //     for pipeline_id in pipelines_building_project {
    //         let event = DeploymentEvent(
    //             revision_id.clone(),
    //             Event::PipelineStarted {
    //                 pipeline_id: pipeline_id.clone(),
    //             },
    //         );
    //         if !is_deployment_complete(&state.db, &event).await? {
    //             info!("Kick-starting pipeline [{}]...", pipeline_id);
    //             state
    //                 .db
    //                 .create_deployment_job_idempotently(&revision_id, &event.1.to_string())
    //                 .await?;
    //         } else {
    //             info!("Skipping stating pipeline [{pipeline_id}], it's already started...");
    //         }
    //     }
    // }

    // Register new artifact
    info!("Registering or updating artifact...");
    let artifact_id = state
        .db
        .get_artifact_by_revision(&revision.0, &project_id, arch.0.as_ref())
        .await?;
    let relative_path = filepath.strip_prefix(&dir).expect(&format!(
        "[{}] was not parent of [{}]",
        dir.display(),
        filepath.display()
    ));

    match artifact_id {
        Some((artifact_id, _, previous_relative_path)) => {
            if previous_relative_path != relative_path {
                let path = dir.join(previous_relative_path);
                info!("Deleting old artifact: {}", path.display());
                tokio::fs::remove_file(&path).await?;
            }

            info!("Updating existing artifact");
            state
                .db
                .update_artifact(&artifact_id, &version.0, &relative_path)
                .await?;
        }
        None => {
            info!("Registering new artifact");
            state
                .db
                .register_artifact(
                    &revision.0,
                    &project_id,
                    arch.0.as_ref(),
                    &version.0,
                    &relative_path,
                )
                .await?;
        }
    }

    // Determine if the revision (with this new artifact) is now activated/immutable
    {
        let artifacts = state.db.list_artifacts_for_revision_id(&revision.0).await?;

        let missing_architectures = state
            .db
            .missing_architectures_for_revision(&revision.0)
            .await?;
        let all_architectures_present = missing_architectures.is_empty();

        let versions = artifacts
            .iter()
            .map(|(_, _, _, version)| version)
            .collect::<HashSet<_>>();
        let all_on_same_version = versions.len() == 1;

        let should_revision_activate = all_on_same_version && all_architectures_present;
        if should_revision_activate {
            info!("Activating revision: {}", &revision.0);
            // This signals the pipeline to continue!
            state.db.activate_revision(&revision.0).await?;
        } else {
            info!(
                "Revision not active: {} versions registered, {} architectures missing",
                versions.len(),
                missing_architectures.len()
            );
        }
    }

    Ok((StatusCode::OK, "Version registered"))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentVariable {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutProjectEnvironmentRequest {
    pub project: String,
    pub environment: String,
    pub variables: Vec<ProjectEnvironmentVariable>,
}

async fn put_project_environment(
    State(state): State<AntZookeeperState>,
    Json(req): Json<PutProjectEnvironmentRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    create_dir_all(envs_persist_dir(&state.root_dir)).await?;
    let envs_file_path = envs_persist_dir(&state.root_dir)
        .join(project_envs_file_name(&req.project, &req.environment));

    let mut file = File::create(envs_file_path)?;

    for variable in req.variables {
        let text = format!("{}=\"{}\"\n", variable.key, variable.value);
        file.write_all(text.as_bytes())?
    }

    Ok((StatusCode::OK, "Project environment registered"))
}

// #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct GetProjectEnvironmentRequest {
//     pub project: String,
//     pub environment: String,
// }

// // #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct GetProjectEnvironmentResponse {
//     pub variables: Vec<ProjectEnvironmentVariable>,
// }
// async fn get_project_environment(
//     State(state): State<AntZookeeperState>,
//     Json(req): Json<GetProjectEnvironmentRequest>,
// ) -> Result<impl IntoResponse, AntZookeeperError> {
//     let envs_file_path =
//         envs_persist_dir(&state.root_dir).join(envs_file_name(&req.project, &req.environment));

//     Ok((StatusCode::OK, Json(GetProjectEnvironmentResponse{variables})))
// }

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertRevisionRequest {
    pub project: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertRevisionResponse {
    pub revision: String,
}
async fn upsert_revision(
    State(state): State<AntZookeeperState>,
    Json(req): Json<UpsertRevisionRequest>,
) -> Result<impl IntoResponse, AntZookeeperError> {
    if !state.db.get_project(&req.project).await? {
        return Err(AntZookeeperError::ResourceNotFound(
            "No such project".to_string(),
        ));
    }

    let revision = state.db.get_latest_revision(&req.project).await?;

    let revision_id = match revision {
        Some((revision_id, None)) => {
            // If there is an "un-activated" revision (not all architectures built on the same version, etc.), return it.
            revision_id
        }

        Some((_, Some(_))) | None => {
            // If this is the first revision for the project ever, make a new one and return it (trivial case)
            let revision_id = state.db.create_revision(&req.project).await?;
            info!("Latest revision activated or non-existent, created new revision: {revision_id}");

            // New revisions are the kick-starter for pipelines that build this project!
            let pipelines_building_project = state
                .db
                .list_deployment_pipelines_building_project(&req.project)
                .await?;
            for pipeline_id in pipelines_building_project {
                let event = DeploymentEvent(
                    revision_id.clone(),
                    Event::PipelineStarted {
                        pipeline_id: pipeline_id.clone(),
                    },
                );
                assert!(!is_deployment_complete(&state.db, &event).await?);

                info!("Kick-starting pipeline [{}]...", pipeline_id);
                state
                    .db
                    .create_deployment(&revision_id, &event.1.to_string())
                    .await?;
            }

            revision_id
        }
    };

    return Ok((
        StatusCode::OK,
        Json(UpsertRevisionResponse {
            revision: revision_id,
        }),
    ));
}

pub fn routes() -> Routes<AntZookeeperState> {
    Routes::new()
        .post("/revision", post(upsert_revision))
        .post("/service", post(register_service))
        .post("/env", post(put_project_environment))
        .post("/artifact", post(register_artifact).layer(
            DefaultBodyLimit::max(1000 * 1000 * 1000), // 1GB
        ))
}
