use crate::config::ProjectConfig;
use crate::link::*;
use crate::ProjectContext;
use anyhow::*;
use log::*;
use std::fs::remove_file;

pub fn prune(ctx: &ProjectContext) -> Result<ProjectConfig> {
    let project = &ctx.project;
    let mut new_project = project.clone();
    new_project.links = new_project
        .links
        .into_iter()
        .filter_map(|mut link| {
            match convert_iter_to_source(link.src.into_iter().filter_map(|x| {
                ctx.project_config_path.join(&x.2).canonicalize().ok()?;
                Some(x)
            })) {
                None => {
                    info!("removing link {}", &link.name);
                    match remove_file(
                        link.destination
                            .to_path_buf(ctx.project.variables.as_ref())
                            .ok()?,
                    ) {
                        Ok(_) => debug!("Successfully removed link {}", &link.name),
                        Err(e) => error!("Failed to remove link {}", e.to_string()),
                    }
                    None
                }
                Some(src) => {
                    link.src = src;
                    Some(link)
                }
            }
        })
        .collect();
    let new_links_len = new_project.links.len();
    let old_links_len = project.links.len();
    if new_links_len == old_links_len {
        debug!("no links removed");
    } else {
        debug!("removed {} links", old_links_len - new_links_len);
    }
    Ok(new_project)
}
