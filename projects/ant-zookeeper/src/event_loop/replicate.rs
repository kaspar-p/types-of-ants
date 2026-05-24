use std::{
    collections::HashMap,
    fs::{exists, File},
    path::PathBuf,
};

use ant_host_agent::client::AntHostAgentClientConfig;
use ant_library::{anthill::AnthillManifest, services::ServiceInstance};
use ant_zookeeper_db::HostGroup;
use anyhow::Context;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use tar::Archive;
use tempfile::tempdir_in;
use tokio::{
    fs::{create_dir_all, OpenOptions},
    io::AsyncWriteExt,
};
use tracing::info;

use crate::{
    fs::{
        artifact_persist_dir, envs_persist_dir, global_envs_file_name, project_envs_file_name,
        secret_file_name, secret_file_path, services_file_name, services_persist_dir,
    },
    state::AntZookeeperState,
};

async fn inject_secrets(
    state: &AntZookeeperState,
    dest: &PathBuf,
    environment: &str,
) -> Result<(), anyhow::Error> {
    info!("Injecting secrets...");
    let manifest = AnthillManifest::from_file(&dest.join("anthill.json"))?;

    let project_secrets_dir = dest.join("secrets");

    // Only make secrets directory if they have any
    if !manifest.secrets.as_deref().unwrap_or_default().is_empty() {
        create_dir_all(&project_secrets_dir).await?;
    }

    for secret in manifest.secrets.unwrap_or_default() {
        let src_file = secret_file_path(&state.root_dir, environment, &secret);
        let dest_file = project_secrets_dir.join(secret_file_name(&secret));
        info!(
            "Copying secret: [{}] -> [{}]",
            src_file.display(),
            dest_file.display()
        );
        tokio::fs::copy(src_file, dest_file).await?;
    }

    Ok(())
}

fn source_env_variables(
    state: &AntZookeeperState,
    service_instance: &ServiceInstance,
    manifest: &AnthillManifest,
    project: &str,
    version: &str,
    environment: &str,
) -> Result<HashMap<String, String>, anyhow::Error> {
    let source_files = vec![
        envs_persist_dir(&state.root_dir).join(global_envs_file_name(&environment)),
        envs_persist_dir(&state.root_dir).join(project_envs_file_name(&project, &environment)),
    ];

    let mut variables = HashMap::<String, String>::new();
    for path in source_files {
        let v = ant_library::env::env_vars_to_map(&path)?;
        variables.extend(v.into_iter());
    }

    variables.insert(
        "PERSIST_DIR".to_string(),
        format!("/home/ant/persist/{project}"),
    );
    variables.insert("SECRETS_DIR".to_string(), "./secrets".to_string());
    variables.insert("VERSION".to_string(), version.to_string());

    // Turn ant-host-agent into ANT_HOST_AGENT, populate *_PORT and *_METRICS_PORT variables.
    let upcase_project = project.to_uppercase().replace("-", "_");
    if let Some(port) = manifest.deployment.as_ref().and_then(|d| d.port) {
        let port_var = format!("{upcase_project}_PORT");

        variables.insert("PORT".to_string(), port.to_string());
        variables.insert(port_var, port.to_string());
    }

    if let Some(metrics_port) = manifest.deployment.as_ref().and_then(|d| d.metrics_port) {
        let metrics_port_var = format!("{upcase_project}_METRICS_PORT");

        variables.insert("METRICS_PORT".to_string(), metrics_port.to_string());
        variables.insert(metrics_port_var, metrics_port.to_string());
    }

    for (k, v) in service_instance
        .additional_vars
        .as_ref()
        .unwrap_or(&HashMap::new())
    {
        variables.insert(k.clone(), v.clone());
    }

    Ok(variables)
}

/// One of the contracts is that docker-compose files that look like:
///     volume: "{{PERSIST_DIR}}/my-dir:/app/dir"
/// will eventually be replaced by the right PERSIST_DIR variable according to the environment:
///     volume: "/persist/my-dir:/app/dir"
/// So we need to mustache-replace the docker-compose.yml file that we find
async fn render_docker_compose(
    state: &AntZookeeperState,
    service_instance: &ServiceInstance,
    manifest: &AnthillManifest,
    dest: &PathBuf,
    project: &str,
    version: &str,
    environment: &str,
) -> Result<(), anyhow::Error> {
    info!("Rendering docker-compose file...");

    let docker_compose_path = dest.join("docker-compose.yml");

    if !exists(&docker_compose_path)? {
        info!("No docker compose file found.");
        return Ok(());
    }

    let template = mustache::compile_path(&docker_compose_path)?;
    let variables = mustache::Data::Map(
        source_env_variables(
            state,
            service_instance,
            manifest,
            project,
            version,
            environment,
        )?
        .into_iter()
        .map(|(k, v)| {
            // The mustache engine takes a line like:
            //  VARIABLE="my-value"
            // and interpolated the quotes into &quot; into the docker-compose file, so remove the leading ones first.
            (
                k,
                v.trim_end_matches("\"")
                    .trim_start_matches("\"")
                    .to_string(),
            )
        })
        .map(|(k, v)| (k, mustache::Data::String(v)))
        .collect(),
    );

    // Create in a temp place in case something fails
    info!("Interpolating variables {:?}", variables);
    let mut docker_compose_file = File::create(dest.join(".__docker-compose.yml"))?;
    template.render_data(&mut docker_compose_file, &variables)?;

    info!("Renaming temp file .__docker-compose.yml to docker-compose.yml");
    std::fs::rename(
        dest.join(".__docker-compose.yml"),
        dest.join("docker-compose.yml"),
    )?;

    Ok(())
}

async fn inject_env_file(
    state: &AntZookeeperState,
    service_instance: &ServiceInstance,
    manifest: &AnthillManifest,
    dest: &PathBuf,
    project: &str,
    version: &str,
    environment: &str,
) -> Result<(), anyhow::Error> {
    info!("Creating .env file...");

    let mut env_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(dest.join(".env"))
        .await?;
    for (key, value) in source_env_variables(
        state,
        service_instance,
        manifest,
        project,
        version,
        environment,
    )? {
        let escaped_value = value.replace(r#"""#, r#"\""#);
        let content = format!("{}=\"{}\"\n", key, escaped_value);
        info!("Writing trimmed config [{}]", content.trim());

        env_file.write_all(content.as_bytes()).await?;
    }

    Ok(())
}

pub async fn replicate_artifact_step(
    state: &AntZookeeperState,
    revision: &str,
    host_group: &HostGroup,
    host: &str,
) -> Result<(), anyhow::Error> {
    let (_, host_arch) = state.db.get_host(&host).await?.expect("host exists");

    let (_, version, artifact_relative_path) = state
        .db
        .get_artifact_by_revision(&revision, &host_group.project, Some(&host_arch))
        .await?
        .unwrap();

    let service_instance = state
        .services
        .service_instance(&host_group.project, host)
        .ok_or(anyhow::Error::msg(format!(
            "cannot deploy to a [{host}] that doesn't have service [{}] defined in services.json!",
            host_group.project
        )))?;

    let artifact_path = artifact_persist_dir(&state.root_dir).join(artifact_relative_path);

    // Construct a new tarball using the build artifact in artifact_path and inject .env and other files.
    let service_file_path = {
        let mut dir = tempdir_in(&state.root_dir.join("tmp"))?;
        dir.disable_cleanup(true);

        // Unpack to a directory
        let unpack_dir_path = {
            let artifact = File::open(&artifact_path)?;
            let gz = GzDecoder::new(&artifact);
            let mut archive = Archive::new(gz);

            let unpack_dir_path = dir.path().join("unpack");
            info!(
                "Unpacking tarball [{}] to [{}]",
                artifact_path.display(),
                unpack_dir_path.display()
            );
            archive.unpack(&unpack_dir_path)?;

            let manifest = AnthillManifest::from_file(&unpack_dir_path.join("anthill.json"))
                .context("failed to find anthill.json in unpacked contents")?;

            inject_env_file(
                state,
                service_instance,
                &manifest,
                &unpack_dir_path,
                &host_group.project,
                &version,
                &host_group.environment,
            )
            .await?;
            inject_secrets(state, &unpack_dir_path, &host_group.environment).await?;
            render_docker_compose(
                state,
                service_instance,
                &manifest,
                &unpack_dir_path,
                &host_group.project,
                &version,
                &host_group.environment,
            )
            .await?;

            unpack_dir_path
        };

        // Create a new tarball with the new files injected
        let pack_file_path = {
            let pack_file_path = dir.path().join("pack.tar");
            info!("Repacking tarball to [{}].", pack_file_path.display());

            let pack_file = File::create_new(&pack_file_path)?;
            let mut archive = tar::Builder::new(GzEncoder::new(pack_file, Compression::default()));

            archive.append_dir_all(".", &unpack_dir_path)?;
            archive.finish()?;

            pack_file_path
        };

        create_dir_all(services_persist_dir(&state.root_dir)).await?;
        let service_file_path = services_persist_dir(&state.root_dir).join(services_file_name(
            &host_group.project,
            Some(&host_arch),
            &version,
        ));
        info!(
            "Copying tarball to final location [{}]",
            service_file_path.display()
        );
        std::fs::copy(pack_file_path, &service_file_path)?;

        service_file_path
    };

    let service_file = File::open(service_file_path)?;

    // Send the service file to ant-host-agent
    let ant_host_agent =
        state
            .ant_host_agent_factory
            .lock()
            .await
            .new_client(AntHostAgentClientConfig {
                endpoint: host.to_string(),
                port: 3232,
            });

    info!("Replicating service file to: {host}");
    ant_host_agent
        .register_service(&host_group.project, &version, service_file)
        .await?;

    info!("Installing service file file to: {host}");
    ant_host_agent
        .install_service(ant_host_agent::routes::service::InstallServiceRequest {
            service_id: host_group.project.clone(),
            version: version.to_string(),
        })
        .await?;

    Ok(())
}
