use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

mod actions;
mod config;
mod file_actions;
mod link;
use config::*;

#[derive(StructOpt)]
#[structopt(about = "Manage dotfiles")]
struct Args {
    #[structopt(short, long)]
    #[structopt(long)]
    config_file: Option<PathBuf>,
    #[structopt(long)]
    project_path: Option<PathBuf>,
    #[structopt(long, short, about = "Locate project from system projects")]
    project: Option<String>,
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
    Manage {
        #[structopt(short, long)]
        default: bool,
    },
    Prune,
    List,
}
fn get_config_loc() -> Option<PathBuf> {
    ProjectDirs::from("com", "AusCyber", "SymSync").map(|x| x.config_dir().to_path_buf())
}

fn get_sys_config(config_path: Option<PathBuf>) -> Result<(PathBuf, SystemConfig)> {
    match config_path {
        Some(x) => Ok((x.clone(), SystemConfig::get_config_file(&x)?)),
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

fn get_project_config(config_path: Option<PathBuf>) -> Result<(PathBuf, ProjectConfig)> {
    match config_path {
        Some(x) => {
            if !x.is_file() {
                Ok((x.clone(), ProjectConfig::get_config_file(&x.join(".links.toml"))?))
            } else {
                Ok((
                    x.parent().context("Could not get parent folder of config file").map(Path::to_path_buf)?,
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
            Ok((proj_path.clone(), ProjectConfig::get_config_file(&file_path)?))
        }
    }
}

pub fn main() -> Result<()> {
    let args = Args::from_args();

    match args {
        Args { command: Command::Sync, project_path, project, system, config_file, .. } => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (path, proj_config) = get_project_config(
                project_path
                    .or_else(|| project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone())))
                    .or(sys_config.default),
            )?;
            actions::sync(proj_config, path, system.context("did not pass system")?)?;
        }
        Args { config_file, project_path, command: Command::Manage { default }, .. } => {
            let (sys_path, sys_config) = get_sys_config(config_file).context("Failure getting system config")?;
            let (proj_path, project) =
                get_project_config(project_path.clone()).context(format!("Failuring getting project {:?}", project_path))?;
            let name = project.name.clone();
            let mut config = actions::manage(sys_config, project, proj_path.clone())
                .context(format!("Failure managing {}", proj_path.clone().display()))?;
            if default {
                config.default = Some(proj_path);
                println!("Set as default");
            }
            fs::write(sys_path, toml::to_vec(&config)?).context("Could not write to system config file")?;
            println!("Managed {}", name);
        }
        Args { system, project_path, project, config_file, command: Command::Add { src, destination, name }, .. } => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (proj_path, project) = get_project_config(
                project_path
                    .or_else(|| project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone())))
                    .or(sys_config.default),
            )
            .context("Could not get project ")?;
            let new_config = actions::add(
                project,
                name.unwrap_or(destination.clone()),
                src.clone(),
                destination.clone(),
                system,
                proj_path.clone(),
            )
            .context("Failure adding link")?;
            let new_toml = toml::to_vec(&new_config)?;
            fs::write(proj_path.join(".links.toml"), new_toml)?;
            println!("Added {}", src.display());
        }
        Args { command: Command::Init { name }, .. } => {
            let dir = env::current_dir()?;
            let project = ProjectConfig::new(
                name.unwrap_or(dir.file_name().and_then(|x| x.to_str()).map(|x| x.into()).context("Invalid name")?),
                &dir,
            );
            let text = toml::to_vec(&project)?;
            fs::write(&dir.join(".links.toml"), &text)?;
        }
        Args { project_path, project, config_file, command: Command::List, .. } => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (_, proj) = get_project_config(
                project_path
                    .or_else(|| project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone())))
                    .or(sys_config.default),
            )?;

            for link in proj.links {
                println!("{:?}", link);
            }
        }
        Args { project_path, command: Command::Revert { file }, .. } => {
            let (proj_path, proj) = get_project_config(project_path)?;
            let config = actions::revert(file, proj, &proj_path)?;
            let text = toml::to_vec(&config)?;
            fs::write(&proj_path.join(".links.toml"), &text)?;

        }
        Args { project_path, project, command: Command::Prune, config_file, .. } => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (proj_path, proj) = get_project_config(
                project_path
                    .or_else(|| project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone())))
                    .or(sys_config.default),
            )?;
            let text = toml::to_vec(&actions::prune(proj_path.clone(), proj))?;
            fs::write(&proj_path.join(".links.toml"), &text)?;
        }
    };

    Ok(())
}
