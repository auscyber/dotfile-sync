use crate::link::{Link, System};
use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
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
    pub fn remove_start(proj_path: &PathBuf, path: &PathBuf) -> Option<String> {
        Some(path.strip_prefix(proj_path).ok()?.to_str()?.to_string())
    }

    pub fn new(name: String, path: &PathBuf) -> ProjectConfig {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        ProjectConfig {
            default: None,
            name,
            id: format!("{}", hasher.finish()),
            systems: Vec::new(),
            links: Vec::new(),
        }
    }

    pub fn get_config_file(path: &PathBuf) -> Result<ProjectConfig> {
        Ok(
            toml::from_slice(&(fs::read(path).context("Project conf doesnt exist")?))
                .context("failed passing config file")?,
        )
    }
}

pub fn get_config_loc() -> Option<PathBuf> {
    ProjectDirs::from("com", "AusCyber", "SymSync").map(|x| x.config_dir().to_path_buf())
}

pub fn get_sys_config<T: AsRef<Path>>(config_path: Option<T>) -> Result<(PathBuf, SystemConfig)> {
    match config_path {
        Some(x) => Ok((x.as_ref().to_path_buf(), SystemConfig::get_config_file(&x)?)),
        None => match get_config_loc()
            .context("Failed to get config location")
            .and_then(|x| Ok(x.join("config.toml").canonicalize()?))
        {
            Ok(x) => Ok((x.clone(), SystemConfig::get_config_file(&x)?)),
            _ => {
                let par_dir = get_config_loc().context("Failed to get config location")?;
                let loc = par_dir.join("config.toml");
                fs::create_dir_all(par_dir)?;
                Ok((loc, SystemConfig::new()))
            }
        },
    }
}

pub fn get_project_config(config_path: Option<PathBuf>) -> Result<(PathBuf, ProjectConfig)> {
    match config_path {
        Some(x) => {
            if !x.is_file() {
                Ok((
                    x.clone(),
                    ProjectConfig::get_config_file(&x.join(".links.toml"))?,
                ))
            } else {
                Ok((
                    x.parent()
                        .context("Could not get parent folder of config file")
                        .map(Path::to_path_buf)?,
                    ProjectConfig::get_config_file(&x)?,
                ))
            }
        }
        None => {
            let proj_path = env::current_dir()?;
            let file_path = proj_path.join(".links.toml");
            if !file_path.exists() {
                bail!("No config file in current directory")
            }
            Ok((
                proj_path.clone(),
                ProjectConfig::get_config_file(&file_path)?,
            ))
        }
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
    pub fn get_config_file<T: AsRef<Path>>(path: &T) -> Result<SystemConfig> {
        let path = path.as_ref();
        let data = fs::read(path).with_context(|| {
            format!(
                "Could not find system config file {}",
                path.clone().display()
            )
        })?;
        Ok(toml::from_slice(&data)?)
    }

    pub fn new() -> SystemConfig {
        SystemConfig {
            default: None,
            projects: HashMap::new(),
        }
    }

    pub fn add_project(&mut self, name: String, path: PathBuf) {
        self.projects
            .insert(name, ProjectOutput { system: None, path });
    }
}
