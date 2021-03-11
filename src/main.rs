use anyhow::{bail, Context, Result};
use clap::{App, Arg, SubCommand};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::{collections::HashMap, hash::Hash};
use std::{env, path};
use std::{fmt::Error, fs, fs::File, path::PathBuf};
use structopt::StructOpt;

type System = String;

fn check_path(path: &PathBuf) -> Result<PathBuf> {
    if path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }
    if path
        .metadata()
        .map(|x| x.permissions().readonly())
        .unwrap_or(false)
    {
        anyhow::bail!("Invalid permissions for file: {}", path.display());
    }

    Ok(path.clone())
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
#[serde(untagged)]
enum Destination {
    DefaultDest(String),
    DynamicDestination(String, HashMap<System, String>),
}

impl Destination {
    pub fn new(project: &ProjectConfig, dest: String) -> Result<Destination> {
        check_path(&project.path.join(dest))?;
        Ok(Destination::DefaultDest(dest))
    }
    pub fn new_dyn(
        project: &ProjectConfig,
        default: String,
        system_map: HashMap<System, String>,
    ) -> Result<Destination> {
        let mut new_map: HashMap<System, String> = system_map
            .iter()
            .map(move |(key, elem)| {
                check_path(&project.path.join(elem))?;
                Ok((key.clone(), elem.clone()))
            })
            .collect::<Result<HashMap<System, String>>>()?;

        Ok(Destination::DynamicDestination(default, new_map))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Link {
    name: String,
    src: PathBuf,
    destination: Destination,
}

impl Link {
    pub fn new(name: String, src: String, destination: Destination) -> Result<Link> {
        Ok(Link {
            name,
            src: check_path(&path::Path::new(&src).to_path_buf())?,
            destination,
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ProjectConfig {
    name: String,
    path: PathBuf,
    id: String,
    systems: Vec<System>,
    links: Vec<Link>,
}

impl ProjectConfig {
    pub fn get_config_file(path: &PathBuf) -> Result<ProjectConfig> {
        let file = File::open(path)?;
        let mut data = Vec::new();
        file.read(&mut data)?;
        Ok(toml::from_slice(&data)?)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct SystemConfig {
    valid_systems: Vec<System>,
    default: Option<PathBuf>,
    projects: Vec<(String, PathBuf)>,
}

impl SystemConfig {
    pub fn get_config_file(path: &PathBuf) -> Result<SystemConfig> {
        let file = File::open(path)?;
        let mut data = Vec::new();
        file.read(&mut data)?;
        Ok(toml::from_slice(&data)?)
    }

    pub fn new() -> SystemConfig {
        SystemConfig {
            valid_systems: Vec::new(),
            default: None,
            projects: Vec::new(),
        }
    }
    pub fn add_project(&mut self, id: String, path: PathBuf) {
        self.projects.push((id, path));
    }
}

#[derive(StructOpt)]
#[structopt(about = "stuff")]
struct Args {
    #[structopt(short, long)]
    debug: bool,
    #[structopt(long, short)]
    config_file: Option<PathBuf>,
    #[structopt(long, short)]
    project: Option<PathBuf>,
    #[structopt(long, short)]
    quick: bool,
    #[structopt(long, short)]
    system: Option<String>,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    Add {
        src: PathBuf,
        destination: String,
        #[structopt(short, long)]
        name: Option<String>,
    },
    Init,
    Revert {
        #[structopt(short, long)]
        file: PathBuf,
    },
    Manage,
}

fn get_config_loc() -> Option<PathBuf> {
    ProjectDirs::from("com", "AusCyber", "SymSync").map(|x| x.config_dir().to_path_buf())
}

mod actions {


    use super::*;

    pub fn manage(sysconfig: SystemConfig, project: ProjectConfig) -> Result<SystemConfig> {
        let mut sysconfig = sysconfig.clone();
        if !project.path.exists() {
            bail!("Project path does not exist");
        }
        sysconfig.add_project(project.id, project.path);
        Ok(sysconfig)
    }

    pub fn add(
        project: ProjectConfig,
        name: String,
        src: PathBuf,
        destination: String,
        system: System,
    ) -> Result<ProjectConfig> {
        let mut project = project.clone();
        let link = Link::new(
            name,
            src.to_str().unwrap().into(),
            Destination::DefaultDest(destination),
        )?;
        project.links.push(link);
        Ok(project)
    }
}

mod file_actions {
    use anyhow::{bail, Result};
    use std::fs;
    use std::path::PathBuf;
    pub fn mv_link(path: &PathBuf, destination: &PathBuf) -> Result<()> {
        let destination = destination.clone();
        if !path.exists() {
            bail!("File does not exist: {}")
        }
        if path.is_file() {
            if destination.is_dir() {
                destination.push(match path.file_name() {
                    Some(x) => x,
                    None => bail!("not a valid file_name"),
                });
            }
            if destination.is_file() {
                bail!("File already exists: {}", destination.display());
            }
            fs::copy(path, destination)?;
            fs::remove_file(path)?;
            fs::soft_link(destination, path.canonicalize()?)?;
        } else if path.is_dir() {
            if destination.is_dir() {
                destination.push(match path.file_name() {
                    Some(x) => x,
                    None => bail!("not a valid file_name"),
                });
            }
            if destination.is_file() {
                bail!("File already exists: {}", destination.display());
            }
            copy_folder(path, &destination)?;
            fs::soft_link(destination, path.canonicalize()?);
            drop((path, destination));
        }
        bail!("File is not file or directory")
    }
    pub fn copy_folder(path: &PathBuf, destination: &PathBuf) -> Result<()> {
        if !path.exists() {
            bail!("Folder does not exist: {}", path.display());
        }
        if !path.is_dir() {
            bail!("Folder isn't actually folder: {}", path.display());
        }
        if !destination.is_file() {
            bail!("Destination is file");
        }
        if !destination.exists() {
            fs::create_dir_all(destination);
        }
        let tru_dest = destination;
        for entry in fs::read_dir(path)? {
            let file = entry?.path();
            let subdest = destination.join(match file.file_name() {
                Some(x) => x,
                None => bail!("could not get file_name"),
            });
            if !file.exists() {
                bail!("File does not exist: {}", file.display());
            }
            if file.is_file() {
                fs::copy(file, subdest)?;
            } else {
                copy_folder(&file, &subdest)?;
            }
        }
        Ok(())
    }
}

fn get_sys_config(config_path: Option<PathBuf>) -> Result<SystemConfig> {
    match config_path {
        Some(x) => SystemConfig::get_config_file(&x),
        None => {
            let loc = match get_config_loc().map(|x| x.join("config.toml")) {
                Some(x) => x,
                None => bail!("Could not locate config file"),
            };
            SystemConfig::get_config_file(&loc)
        }
    }
}

fn get_project_config(config_path: Option<PathBuf>) -> Result<ProjectConfig> {
    match config_path {
        Some(mut x) => {
            if !x.is_file() {
                x.push(".links.toml")
            }
            ProjectConfig::get_config_file(&x)
        }
        None => {
            ProjectConfig::get_config_file(&(PathBuf::new().join(".links.toml").canonicalize()?))
                .context("Failure using default Config")
        }
    }
}

fn main() -> Result<()> {
    let args = Args::from_args();

    match args {
        Args {
            system,
            project,
            command:
                Command::Add {
                    src,
                    destination,
                    name,
                },
            ..
        } => {
            let new_config = actions::add(get_project_config(project)?, name.unwrap_or(destination), src, destination, system.unwrap())?;
            let new_toml = toml::to_vec(&new_config)?;
        },
        _ => (),
    };

    let config_file = if let Some(proj_dir) = ProjectDirs::from("com", "AusCyber", "SymSync") {
        let conf_dir = proj_dir.config_dir().to_path_buf();

        if !conf_dir.exists() {
            fs::create_dir_all(&conf_dir)?;
        }
        let config_file_loc = conf_dir.join("config.toml");

        let f = fs::File::create(config_file_loc)?;
    };

    Ok(())
}
