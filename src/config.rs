use crate::link::{Link, System};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub id: String,
    pub default: Option<System>,
    pub systems: Vec<System>,
    pub links: Vec<Link>,
}
impl ProjectConfig {
    pub fn new(name: String, path: &PathBuf) -> ProjectConfig {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        ProjectConfig { default: None, name, id: format!("{}", hasher.finish()), systems: Vec::new(), links: Vec::new() }
    }

    pub fn get_config_file(path: &PathBuf) -> Result<ProjectConfig> {
        Ok(toml::from_slice(&(fs::read(path).context("Project conf doesnt exist")?)).context("failed passing config file")?)
    }
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectOutput {
    pub system: Option<System>,
    pub path: PathBuf,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SystemConfig {
    pub default: Option<PathBuf>,
    pub projects: HashMap<String, ProjectOutput>,
}

impl SystemConfig {
    pub fn get_config_file(path: &PathBuf) -> Result<SystemConfig> {
        let data = fs::read(path).with_context(|| format!("Could not find system config file {}", path.clone().display()))?;
        Ok(toml::from_slice(&data)?)
    }

    pub fn new() -> SystemConfig {
        SystemConfig { default: None, projects: HashMap::new() }
    }

    pub fn add_project(&mut self, name: String, path: PathBuf) {
        self.projects.insert(name, ProjectOutput { system: None, path });
    }
}
