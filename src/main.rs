use anyhow::{bail, Result};
use clap::{App, Arg, SubCommand};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Read, Write};
use std::{collections::HashMap, hash::Hash};
use std::{fmt::Error, fs, fs::File, path::PathBuf};
use structopt::StructOpt;

type System = String;

fn check_path(_path: String) -> Result<PathBuf> {
    let path: PathBuf = PathBuf::from(&_path);

    if path.exists() {
        anyhow::bail!("File does not exist: {}", _path);
    }
    if path
        .metadata()
        .map(|x| x.permissions().readonly())
        .unwrap_or(false)
    {
        anyhow::bail!("Invalid permissions for file: {}", _path);
    }

    Ok(path)
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
#[serde(untagged)]
enum Destination {
    DefaultDest(PathBuf),
    DynamicDestination(PathBuf, HashMap<System, PathBuf>),
}

impl Destination {
    pub fn new(dest: String) -> Result<Destination> {
        Ok(Destination::DefaultDest(check_path(dest)?))
    }
    pub fn new_dyn(default: String, system_map: HashMap<System, String>) -> Result<Destination> {
        let mut new_map: HashMap<System, PathBuf> = system_map
            .iter()
            .map(|(key, elem)| Ok((key.clone(), check_path(elem.clone())?)))
            .collect::<Result<HashMap<System, PathBuf>>>()?;

        Ok(Destination::DynamicDestination(
            check_path(default)?,
            new_map,
        ))
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
            src: check_path(src)?,
            destination,
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct ProjectConfig {
    name: String,
    id: String,
    systems: Vec<System>,
    links: Vec<Link>,
}
#[derive(Deserialize, Serialize, Debug)]
struct SystemConfig {
    valid_systems: Vec<System>,
    default: Option<PathBuf>,
    projects: Vec<(String, PathBuf)>,
}

impl SystemConfig {
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
#[structopt(about = "the stupid content tracker")]
struct Args {
    #[structopt(long, short)]
    config_file: Option<PathBuf>,
    #[structopt(long, short)]
    project: Option<PathBuf>,
    #[structopt(long, short, default_value = false)]
    quick: Bool,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    Add {
        #[structopt(short, long)]
        file: PathBuf,
    },
    Revert {
        #[structopt(short, long)]
        file: PathBuf,
    },
    Manage {
        #[structopt(short)]
        system: Option<System>,
    },
}

fn get_config_loc() -> Option<PathBuf> {
    ProjectDirs::from("com", "AusCyber", "SymSync").map(|x| x.config_dir().to_path_buf())
}

fn manage(sysconfig: SystemConfig, project: ProjectConfig) -> Result<SystemConfig> {
    //TODO
}





fn get_config_file<'a,T : Deserialize<'a>>(path : &PathBuf) -> Result<T>{
    let file = File::open(&path)?;
    let mut data = Vec::new();
    file.read(&mut data)?;
    Ok(toml::from_slice(&data)?)
}



fn main() -> Result<()> {
    let args = Args::from_args();

    match args {
        Args {
            config_file : None,
            project: None,
            command: Command::Manage { system: None },
            ..
        } => {
        }

        _ => (),
    }

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
