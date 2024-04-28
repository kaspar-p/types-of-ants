use std::str::FromStr;

use ant_metadata::Project;
use cargo_metadata::MetadataCommand;

fn generate_dependency_block(dependency: Project) -> String {
    let dep_name: String = String::from(dependency.as_str());
    let s = format!(
        "# Generate build cache for {dep_name}
RUN cargo new {dep_name}
WORKDIR /types-of-ants/{dep_name}
COPY ./projects/{dep_name}/Cargo.toml ../{dep_name}/Cargo.lock ./
RUN cargo build --release"
    );
    return String::from(s);
}

fn find_dependencies(current: Project) -> Vec<Project> {
    let metadata = MetadataCommand::new().exec().unwrap();
    let p = metadata
        .packages
        .iter()
        .find(|p| {
            println!("{}", p.name);
            p.name == current.as_str()
        })
        .expect(format!("Package {} not found in workspace!", current.as_str()).as_str());

    let local_deps = p
        .dependencies
        .iter()
        .filter(|&dep| dep.path.is_some())
        .map(|dep| match Project::from_str(dep.name.as_str()) {
            Err(e) => panic!(
                "Project {:#?} is a local dependency, but not in the Project enum!",
                e
            ),
            Ok(p) => p,
        })
        .collect::<Vec<Project>>();

    return local_deps;
}

/// Currently assumes all of a projects dependencies are just Cargo dependencies, and nothing else.
pub fn generate_dockerfile(current: Project) -> String {
    let mut f = format!(
        "
FROM rust:1.66.0 AS build
WORKDIR /types-of-ants

RUN rustup target add x86_64-unknown-linux-musl
"
    );

    let deps = find_dependencies(current);
    for dep in deps {
        f.push_str(&generate_dependency_block(dep));
        f.push_str("\n");
    }

    f.push_str(&format!(
        "
FROM scratch
COPY --from=build /types-of-ants/cargo/bin/{} .
USER 1000
CMD [\"./{}\"]",
        current.as_str().to_owned(),
        current.as_str().to_owned(),
    ));

    return f;
}

#[cfg(test)]
mod test {
    use super::find_dependencies;
    use ant_metadata::Project;

    #[test]
    fn ant_who_tweets_depends_on_ant_data_farm() {
        let deps = find_dependencies(Project::AntWhoTweets);
        let found = deps.iter().find(|&dep| match dep {
            Project::AntDataFarm => true,
            _ => false,
        });

        assert!(found.is_some());
    }
}
