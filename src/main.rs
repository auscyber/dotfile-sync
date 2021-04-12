use anyhow::{Context, Result};
use log::*;
use std::{env, fs, path::PathBuf};
use structopt::StructOpt;

mod actions;
mod config;
mod file_actions;
mod link;
mod util;
use config::*;
use link::System;

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
    system: Option<System>,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    Sync,
    Add {
        src: PathBuf,
        destination: Option<String>,
        #[structopt(short, long)]
        name: Option<String>,
    },
    Init {
        name: Option<String>,
    },
    Revert {
        file: PathBuf,
    },
    Manage {
        #[structopt(short, long)]
        default: bool,
    },
    Prune,
    List,
}

pub fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let Args {
        project_path,
        project,
        system,
        config_file,
        command,
    } = Args::from_args();

    match command {
        Command::Sync => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (path, proj_config) = get_project_config(
                project_path
                    .or_else(|| {
                        project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone()))
                    })
                    .or(sys_config.default),
            )?;
            actions::sync(proj_config, path, system)?;
        }
        Command::Manage { default } => {
            let (sys_path, sys_config) =
                get_sys_config(config_file).context("Failure getting system config")?;
            let (proj_path, project) = get_project_config(project_path.clone())
                .context(format!("Failuring getting project {:?}", project_path))?;
            let name = project.name.clone();
            let mut config = actions::manage(sys_config, project, proj_path.clone())
                .context(format!("Failure managing {}", proj_path.clone().display()))?;
            if default {
                config.default = Some(proj_path);
                info!("Set as default");
            }
            fs::write(sys_path, toml::to_vec(&config)?)
                .context("Could not write to system config file")?;
            info!("Managed {}", name);
        }
        Command::Add {
            src,
            destination,
            name,
        } => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (proj_path, project) = get_project_config(
                project_path
                    .or_else(|| {
                        project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone()))
                    })
                    .or(sys_config.default),
            )
            .context("Could not get project ")?;
            let new_config = actions::add(
                project,
                name.or(destination.clone()).unwrap_or(
                    proj_path
                        .file_name()
                        .and_then(|x| x.to_str())
                        .unwrap()
                        .into(),
                ),
                src.clone(),
                destination.clone(),
                system,
                proj_path.clone(),
            )
            .context("Failure adding link")?;
            let new_toml = toml::to_vec(&new_config)?;
            fs::write(proj_path.join(".links.toml"), new_toml)?;
            info!("Added {}", src.display());
        }
        Command::Init { name } => {
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
        Command::List => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (_, proj) = get_project_config(
                project_path
                    .or_else(|| {
                        project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone()))
                    })
                    .or(sys_config.default),
            )?;

            for link in proj.links {
                println!("{:?}", link);
            }
        }
        Command::Revert { file } => {
            let (proj_path, proj) = get_project_config(
                project_path
                    .or_else(|| {
                        project.and_then(|y| {
                            get_sys_config(config_file.clone())
                                .ok()?
                                .1
                                .projects
                                .get(&y)
                                .map(|x| x.path.clone())
                        })
                    })
                    .or_else(|| get_sys_config(config_file.clone()).ok()?.1.default),
            )
            .context("Could not find project_path")?;
            let system = system
                .or_else(|| {
                    get_sys_config(config_file)
                        .ok()?
                        .1
                        .projects
                        .get(&proj.name)?
                        .clone()
                        .system
                })
                .or(proj.default.clone());
            let config = actions::revert(file, proj, &proj_path, system)?;
            let text = toml::to_vec(&config)?;
            fs::write(&proj_path.join(".links.toml"), &text)?;
        }
        Command::Prune => {
            let (_, sys_config) = get_sys_config(config_file)?;
            let (proj_path, proj) = get_project_config(
                project_path
                    .or_else(|| {
                        project.and_then(|y| sys_config.projects.get(&y).map(|x| x.path.clone()))
                    })
                    .or(sys_config.default),
            )?;
            let text = toml::to_vec(&actions::prune(proj_path.clone(), proj))?;
            fs::write(&proj_path.join(".links.toml"), &text)?;
        }
    };

    Ok(())
}
