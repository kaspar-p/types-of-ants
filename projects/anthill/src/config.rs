use ant_metadata::Project;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Represents the requirement of an artifact from one project to another
// struct Requirement {
//     /// The project that vends the artifact
//     source: Project,

//     /// The `name` of the artifact given
//     artifact: String,

//     /// The destination to save, relative to the binary, of the requirement.
//     /// For example, if the binary is at `~/my_binary`, and there is a requirement with the destination
//     /// of `"./my_config_file.txt", the file will be saved at `~/my_config_file.txt`.
//     destination: String,
// }

#[derive(Debug, Serialize, Deserialize)]
pub enum EnvironmentKey {
    Database,
    Twitter,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Dependency {
    /// A dependency that comes from the build artifacts of a project.
    /// For example, the `ant-on-the-web` webserver relies on the statically generated HTML
    /// from `ant-you-can-see`, the React frontend project. So, `ant-on-the-web` would have a
    /// ProjectSource dependency on `ant-you-can-see`, with whatever name that `ant-you-can-see`
    /// decides to vend its artifacts at.
    ///
    /// Concretely, if `ant-you-can-see` had an `anthill.json` configured as:
    /// ```json
    /// {
    ///   ...
    ///   artifacts: [
    ///     { name: "static-output", ... },
    ///   ]
    ///   ...
    /// }
    /// ```
    /// and `ant-on-the-web` requires those static HTML files to appear at ./static/index.html,
    /// relative to the binary, the `anthill.json` for `ant-on-the-web` would look like:
    /// ```json
    /// {
    ///   ...
    ///   requires: [
    ///     {
    ///       source: "ant-you-can-see",
    ///       artifact: "static-output",
    ///       destination: "static"
    ///     }
    ///   ]
    /// }
    ///
    /// ```
    ///
    ProjectSource {
        source: Project,
        artifact: String,
        destination: String,
    },

    /// A dependency on some sort of environment being present. Commonly, environment variables
    /// being present in the form of a .env or .env.local file. I'm not yet sure from where to
    /// "get" these.
    EnvironmentSource { keys: Vec<EnvironmentKey> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Requires {
    /// The static artifacts that need to be there, as artifacts
    pub fixed: Vec<Dependency>,

    /// A list of artifacts required at runtime. These are commonly artifacts that can't be committed to source code,
    /// like environment variable secrets, or .env files.
    /// The source of these files is still undetermined, there might be a central source for all that.
    pub secret: Option<Vec<Dependency>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BuildKind {
    /// Represents a rust binary artifact. Assumes to be using cargo,
    /// and assumes that `cargo build` will build the binary.
    #[serde(rename = "rust-bin")]
    RustBin,

    /// Static files, whose mere presence is required. For example,
    /// this artifact is the type vended by `ant-you-can-see`, the
    /// React frontend project.
    #[serde(rename = "next-frontend-export")]
    NextFrontendExport,
}

impl BuildKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildKind::RustBin => "rust-bin",
            BuildKind::NextFrontendExport => "next-frontend-export",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub name: String,
    pub kind: BuildKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// The name of the project, e.g. 'anthill' or 'ant-data-farm'
    pub project: String,

    /// The SemVer version of the project, beginning at '1.0.0'. Automatically bumped on commit
    /// Gotta work on that tho, it's not a thing yet
    pub version: Version,

    /// The artifacts that the current project produces, e.g. as standalone binaries, vended
    /// files, etc.
    pub artifacts: Vec<Product>,

    /// A list of the required artifacts, necessary for a successful build
    pub requires: Option<Requires>,
}
