use crate::{config::*, file_actions, link::*};
use anyhow::{bail, Context, Result};
use std::{fs, path::PathBuf};
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
            fs::soft_link(destination, &x.src).context(format!("Failed linking {}", &x.name))?;
            Ok(())
        })
        .collect::<Result<_>>()
}

pub fn manage(sysconfig: SystemConfig, project: ProjectConfig, project_path: PathBuf) -> Result<SystemConfig> {
    let mut sysconfig = sysconfig.clone();
    if !project_path.exists() {
        bail!("Project path does not exist");
    }
    sysconfig.add_project(project.name, project_path);
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
    let link = Link::new(name, src.to_str().unwrap().into(), Destination::DefaultDest(destination.clone()))?;
    file_actions::mv_link(&src, &project_path.join(&destination)).context("Failure linking")?;
    project.links.push(link);
    Ok(project)
}
