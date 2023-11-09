use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum ArtifactSelection {
    Latest,
    SpecificVersion(i32),
}

impl ArtifactSelection {
    pub fn as_str(&self) -> String {
        match self {
            ArtifactSelection::Latest => "latest".to_owned(),
            ArtifactSelection::SpecificVersion(v) => format!("1.0.{v}"),
        }
    }
}

impl Into<reqwest::Body> for ArtifactSelection {
    fn into(self) -> reqwest::Body {
        reqwest::Body::from(self.as_str())
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum Architecture {
    RaspberryPi,
    Mac,
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::RaspberryPi => "rpi",
            Architecture::Mac => "mac",
        }
    }
}

impl Into<reqwest::Body> for Architecture {
    fn into(self) -> reqwest::Body {
        reqwest::Body::from(self.as_str())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone)]
pub enum Project {
    AntBuildingProjects,
    AntDataFarm,
    AntGateway,
    AntHostAgent,
    AntJustCheckingIn,
    AntMetadata,
    AntOnTheWeb,
    AntOwningArtifacts,
    AntWhoTweets,
    Anthill,
}

impl Into<reqwest::Body> for Project {
    fn into(self) -> reqwest::Body {
        reqwest::Body::from(self.as_str())
    }
}

impl FromStr for Project {
    type Err = ();
    fn from_str(input: &str) -> Result<Project, Self::Err> {
        match input {
            "ant-building-projects" => Ok(Project::AntBuildingProjects),
            "AntBuildingProjects" => Ok(Project::AntBuildingProjects),

            "ant-data-farm" => Ok(Project::AntDataFarm),
            "AntDataFarm" => Ok(Project::AntDataFarm),

            "ant-gateway" => Ok(Project::AntGateway),
            "AntGateway" => Ok(Project::AntGateway),

            "ant-host-agent" => Ok(Project::AntHostAgent),
            "AntHostAgent" => Ok(Project::AntHostAgent),

            "ant-just-checking-in" => Ok(Project::AntJustCheckingIn),
            "AntJustCheckingIn" => Ok(Project::AntJustCheckingIn),

            "ant-metadata" => Ok(Project::AntMetadata),
            "AntOnTheWeb" => Ok(Project::AntMetadata),

            "ant-on-the-web" => Ok(Project::AntOnTheWeb),
            "AntOnTheWeb" => Ok(Project::AntOnTheWeb),

            "ant-owning-artifacts" => Ok(Project::AntOwningArtifacts),
            "AntOwningArtifacts" => Ok(Project::AntOwningArtifacts),

            "ant-who-tweets" => Ok(Project::AntWhoTweets),
            "AntWhoTweets" => Ok(Project::AntWhoTweets),

            "anthill" => Ok(Project::Anthill),
            "Anthill" => Ok(Project::Anthill),

            _ => Err(()),
        }
    }
}

impl Project {
    pub fn as_str(&self) -> &'static str {
        match self {
            Project::AntBuildingProjects => "ant-building-projects",
            Project::AntDataFarm => "ant-data-farm",
            Project::AntGateway => "ant-gateway",
            Project::AntHostAgent => "ant-host-agent",
            Project::AntJustCheckingIn => "ant-just-checking-in",
            Project::AntMetadata => "ant-metadata",
            Project::AntOnTheWeb => "ant-on-the-web",
            Project::AntOwningArtifacts => "ant-owning-artifacts",
            Project::AntWhoTweets => "ant-who-tweets",
            Project::Anthill => "anthill",
        }
    }
}
