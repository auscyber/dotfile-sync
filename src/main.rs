#![feature(result_flattening)]
#![feature(in_band_lifetimes)]
use anyhow::{Context, Result};
use clap_generate::{generate, Shell};
use log::*;
use std::{
    env,
    path::{Path, PathBuf},
};
//use structopt::StructOpt;

use clap::{IntoApp, Parser};
use colored::*;
use std::convert::TryInto;

mod actions;
mod config;
mod file_actions;
mod goals;
mod link;
mod packages;
#[cfg(test)]
mod tests;
mod util;

use config::*;
use link::{Link, System};
use util::WritableConfig;

#[derive(Parser, Clone)]
#[clap(about = "Manage dotfiles")]
pub struct Args {
    #[clap(short, long, about = "Location of system config file", global = true)]
    config_file: Option<PathBuf>,
    #[clap(long, global = true, about = "Location of project config file")]
    project_path: Option<PathBuf>,
    #[clap(
        long,
        short,
        about = "Locate project from system projects",
        global = true
    )]
    project: Option<String>,
    #[clap(long, short, global = true)]
    system: Option<System>,
    #[clap(subcommand)]
    command: Command,
}

pub struct ProjectContext {
    pub args: Args,
    pub project: ProjectConfig,
    pub project_config_path: PathBuf,
    pub system_config: SystemConfig,
    pub system_config_path: PathBuf,
    pub system: Option<System>,
}

impl ProjectContext {
    pub fn get_link_for_file<'a>(&'a mut self, file: &Path) -> Option<&'a mut Link> {
        let stripped_path = file.to_str()?;
        self.project
            .links
            .iter_mut()
            .find(|x| x.src.contains_path(stripped_path))
    }

    pub fn in_project(&self, path: &str) -> Result<bool> {
        Ok(self
            .project_config_path
            .join(path)
            .canonicalize()
            .map(|x| x.exists())
            .unwrap_or(false)
            || self.project.links.iter().any(|x| x.src.contains_path(path)))
    }
}

impl TryInto<ProjectContext> for Args {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<ProjectContext> {
        let (system_config_file, system_config) = get_sys_config(self.config_file.as_ref())?;
        let current = std::env::current_dir()?;
        let (path, proj_config) = get_project_config(
            self.project_path
                .as_ref()
                .or_else(|| {
                    if current.join(".links.toml").exists() {
                        Some(&current)
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    self.project
                        .clone()
                        .and_then(|y| system_config.projects.get(&y))
                        .map(|x| &x.path)
                })
                .or(system_config.default.as_ref()),
        )?;

        let system = self
            .system
            .as_ref()
            .or_else(|| {
                system_config
                    .get_project(&proj_config.name)?
                    .system
                    .as_ref()
            })
            .or(proj_config.default.as_ref())
            .cloned();
        Ok(ProjectContext {
            //            command: self.command.clone(),
            args: self,
            project: proj_config,
            project_config_path: path,
            system_config,
            system_config_path: system_config_file,
            system,
        })
    }
}

impl Args {
    fn try_to_context(self) -> Result<ProjectContext> {
        self.try_into()
    }
}

#[derive(Parser, Clone)]
enum Command {
    #[clap(about = "Link all files in project")]
    Sync {
        #[clap(long = "installed-programs")]
        installed_programs: bool,
        #[clap(short = 'g')]
        goal: Option<String>,
    },
    #[clap(about = "Move and link project")]
    Add {
        src: Vec<String>,
        #[clap(short, long)]
        destination: Option<String>,
        #[clap(short, long)]
        name: Option<String>,
    },
    #[clap(about = "Initalise project")]
    Init { name: Option<String> },
    #[clap(about = "Revert path")]
    Revert { file: PathBuf },
    #[clap(about = "Add project to system configuration")]
    Manage {
        #[clap(short, long)]
        default: bool,
    },
    #[clap(about = "Prune all removed files in the project")]
    Prune,
    #[clap(about = "Work with Goals", subcommand)]
    Goals(actions::goal::GoalSubCommand),
    Completion {
        #[clap(long, value_name = "SHELL", arg_enum)]
        shell: Shell,
    },
    #[clap(about = "List all links in the project")]
    List,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    match args.command.clone() {
        Command::Completion { shell } => {
            generate(shell, &mut Args::into_app(), "dots", &mut std::io::stdout())
        }
        Command::Sync {
            goal,
            installed_programs,
        } => {
            actions::sync(args.try_into()?, goal, installed_programs).await?;
        }
        Command::Manage { default } => {
            let ctx = args.try_to_context()?;
            let config = actions::manage(&ctx, default).context(format!(
                "Failure managing {}",
                ctx.project_config_path.display()
            ))?;
            config.write_to_file(&ctx.system_config_path)?;
            info!("Managed {}", ctx.project.name);
        }
        Command::Add {
            src,
            destination,
            name,
        } => {
            let ctx = args.try_to_context()?;
            let config = actions::add(&ctx, src, destination, name)
                .await
                .context("Failure adding link")?;
            config.save(&ctx)?;
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
            project.write_to_file(&dir.join(".links.toml"))?;
        }
        Command::List => {
            let ctx = args.try_to_context()?;
            println!("{} {}", "Links for".bold(), ctx.project.name.bold());
            for link in ctx.project.links {
                print!("{}", link);
            }
        }
        Command::Revert { file } => {
            let ctx = args.try_to_context()?;
            let config = actions::revert(&ctx, &file).await?;
            config.save(&ctx)?;
        }
        Command::Prune => {
            let ctx = args.try_to_context()?;
            actions::prune(&ctx)?.save(&ctx)?;
        }
        Command::Goals(command) => {
            let ctx = args.try_to_context()?;
            let config = actions::goal::goals(&ctx, command).await?;
            config.save(&ctx)?;
        }
    };
    Ok(())
}
