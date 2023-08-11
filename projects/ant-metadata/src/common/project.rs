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
    AntOnTheWeb,
    AntOwningArtifacts,
    AntHostAgent,
    AntGateway,
    AntDataFarm,
    AntWhoTweets,
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
            "ant-on-the-web" => Ok(Project::AntOnTheWeb),
            "AntOnTheWeb" => Ok(Project::AntOnTheWeb),

            "ant-owning-artifacts" => Ok(Project::AntOwningArtifacts),
            "AntOwningArtifacts" => Ok(Project::AntOwningArtifacts),

            "ant-host-agent" => Ok(Project::AntHostAgent),
            "AntHostAgent" => Ok(Project::AntHostAgent),

            "ant-gateway" => Ok(Project::AntGateway),
            "AntGateway" => Ok(Project::AntGateway),

            "ant-data-farm" => Ok(Project::AntDataFarm),
            "AntDataFarm" => Ok(Project::AntDataFarm),

            "ant-who-tweets" => Ok(Project::AntWhoTweets),
            "AntWhoTweets" => Ok(Project::AntWhoTweets),
            _ => Err(()),
        }
    }
}

impl Project {
    pub fn as_str(&self) -> &'static str {
        match self {
            Project::AntOnTheWeb => "ant-on-the-web",
            Project::AntOwningArtifacts => "ant-owning-artifacts",
            Project::AntGateway => "ant-gateway",
            Project::AntDataFarm => "ant-data-farm",
            Project::AntWhoTweets => "ant-who-tweets",
            Project::AntHostAgent => "ant-host-agent",
        }
    }
}
