use actions::manage;
use anyhow::{bail, Context, Result};
use clap::{App, Arg, SubCommand};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    hash::Hasher,
    io::{Read, Write},
    path::Path,
};
use std::{collections::HashMap, hash::Hash};
use std::{env, path};
use std::{fmt::Error, fs, fs::File, path::PathBuf};
use structopt::StructOpt;

type System = String;

fn check_path(path: &PathBuf) -> Result<PathBuf> {
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }
    if path
        .metadata()
        .map(|x| x.permissions().readonly())
        .unwrap_or(true)
    {
        anyhow::bail!("Invalid permissions for file: {}", path.display());
    }

    Ok(path.clone())
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
#[serde(untagged)]
enum Destination {
    DefaultDest(String),
    DynamicDestinationWithDefault(System, HashMap<System, String>),
    DynamicDestination(HashMap<System, String>),
}

impl Destination {
    pub fn new(base_url: &PathBuf, dest: String) -> Result<Destination> {
        check_path(&base_url.join(&dest))?;
        Ok(Destination::DefaultDest(dest.clone()))
    }
    pub fn with_default(
        base_url: &PathBuf,
        default: String,
        system_map: HashMap<System, String>,
    ) -> Result<Destination> {
        let mut new_map: HashMap<System, String> = system_map
            .iter()
            .map(move |(key, elem)| {
                check_path(&base_url.join(elem))?;
                Ok((key.clone(), elem.clone()))
            })
            .collect::<Result<HashMap<System, String>>>()?;

        Ok(Destination::DynamicDestinationWithDefault(default, new_map))
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
pub struct ProjectConfig {
    name: String,
    id: String,
    systems: Vec<System>,
    links: Vec<Link>,
}

impl ProjectConfig {
    pub fn new(name: String, path: &PathBuf) -> ProjectConfig {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        ProjectConfig {
            name,
            id: format!("{}", hasher.finish()),
            systems: Vec::new(),
            links: Vec::new(),
        }
    }
    pub fn get_config_file(path: &PathBuf) -> Result<ProjectConfig> {
        Ok(toml::from_slice(&(fs::read(path)?)).context("failed passing config file")?)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SystemConfig {
    valid_systems: Vec<System>,
    default: Option<PathBuf>,
    projects: Vec<(String, PathBuf)>,
}

impl SystemConfig {
    pub fn get_config_file(path: &PathBuf) -> Result<(SystemConfig)> {
        let mut file = File::open(path)?;
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
    Sync,
    Add {
        src: PathBuf,
        destination: String,
        #[structopt(short, long)]
        name: Option<String>,
    },
    Init {
        name: Option<String>,
    },
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

    pub fn sync(project: ProjectConfig, path: PathBuf, system: System) -> Result<()> {
        project
            .links
            .iter()
            .map(|x| {
                let destination = path.join(match x.destination.clone() {
                    Destination::DefaultDest(y) => y,
                    Destination::DynamicDestination(y) => {
                        if let Some(a) = y.get(&system) {
                            a.clone()
                        } else {
                            return Ok(());
                        }
                    }
                    Destination::DynamicDestinationWithDefault(a, map) => {
                        if let Some(b) = map.get(&system).or_else(|| map.get(&a)) {
                            b.clone()
                        } else {
                            return Ok(());
                        }
                    }
                });
                println!("Linking {}", x.name);
                fs::soft_link(destination, &x.src)
                    .context(format!("Failed linking {}", &x.name))?;
                Ok(())
            })
            .collect::<Result<_>>()
    }

    pub fn manage(
        sysconfig: SystemConfig,
        project: ProjectConfig,
        project_path: PathBuf,
    ) -> Result<SystemConfig> {
        let mut sysconfig = sysconfig.clone();
        if !project_path.exists() {
            bail!("Project path does not exist");
        }
        sysconfig.add_project(project.id, project_path);
        println!("Successfully managed {}", project.name);
        Ok(sysconfig)
    }

    pub fn add(
        project: ProjectConfig,
        name: String,
        src: PathBuf,
        destination: String,
        system: Option<System>,
        project_path: PathBuf,
    ) -> Result<ProjectConfig> {
        let mut project = project.clone();
        let link = Link::new(
            name,
            src.to_str().unwrap().into(),
            Destination::DefaultDest(destination.clone()),
        )?;
        file_actions::mv_link(&src, &project_path.join(&destination)).context("Failure linking")?;
        project.links.push(link);
        Ok(project)
    }
}

mod file_actions {
    use anyhow::{bail, Result};
    use std::fs;
    use std::path::PathBuf;
    pub fn mv_link(path: &PathBuf, destination: &PathBuf) -> Result<()> {
        let mut destination = destination.clone();
        if destination.exists() && destination.is_file() {
            bail!("File already exists {}", destination.display());
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
            let full_path = path.canonicalize()?;
            fs::copy(path, &destination)?;
            fs::remove_file(path)?;
            fs::soft_link(destination, full_path)?;
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
            fs::remove_dir(path)?;
            fs::soft_link(&destination, path.canonicalize()?)?;
            drop((path, destination));
        } else {
            bail!("File is not file or directory")
        }
        Ok(())
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
            fs::create_dir_all(destination)?;
        }
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
        None => match get_config_loc().map(|x| x.join("config.toml")) {
            Some(x) => SystemConfig::get_config_file(&x),
            None => Ok(SystemConfig::new()),
        },
    }
}

fn get_project_config(config_path: Option<PathBuf>) -> Result<(PathBuf, ProjectConfig)> {
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
                        .context("Could not get parent folder ofconfig file")
                        .map(Path::to_path_buf)?,
                    ProjectConfig::get_config_file(&x)?,
                ))
            }
        }
        None => {
            let proj_path = env::current_dir()?;
            Ok((
                proj_path.clone(),
                ProjectConfig::get_config_file(&proj_path.join(".links.toml"))?,
            ))
        }
    }
}

fn main() -> Result<()> {
    let args = Args::from_args();

    match args {
        Args {
            command: Command::Sync,
            project,
            system,
            config_file,
            ..
        } => {
            let sys_config = get_sys_config(config_file)?;
            let (path, proj_config) = get_project_config(project.or(sys_config.default))?;
            actions::sync(proj_config, path, system.context("did not pass system")?)?;
        }
        Args {
            config_file,
            project,
            command: Command::Manage,
            ..
        } => {
            let sys_config =
                get_sys_config(config_file).context("Failure getting system config")?;
            let (proj_path, project) = get_project_config(project.clone())
                .context(format!("Failuring getting project {:?}", project))?;
            let name = project.name.clone();
            manage(sys_config, project, proj_path.clone())
                .context(format!("Failure managing {}", proj_path.clone().display()))?;
            println!("Managed {}", name);
        }
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
            let proj_path = project.clone().unwrap_or(env::current_dir()?);
            let new_config = actions::add(
                get_project_config(project)?.1,
                name.unwrap_or(destination.clone()),
                src,
                destination.clone(),
                system,
                proj_path.clone(),
            )
            .context("Failure adding link")?;
            let new_toml = toml::to_vec(&new_config)?;
            fs::write(proj_path.join(".links.toml"), new_toml)?;
        }
        Args {
            command: Command::Init { name },
            ..
        } => {
            let dir = env::current_dir()?;
            let project = ProjectConfig::new(
                name.unwrap_or(
                    dir.file_name()
                        .and_then(|x| x.to_str())
                        .map(|x| x.into())
                        .context("Invalid name")?,
                ),
                &dir,
            );
            let text = toml::to_vec(&project)?;
            fs::write(&dir.join(".links.toml"), &text)?;
        }
        _ => (),
    };

    Ok(())
}
