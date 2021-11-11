use crate::{link::Link, ProjectContext};
use anyhow::{Context, Result};
use log::*;
use std::sync::Arc;
use tokio::fs;

pub async fn sync(
    ctx: ProjectContext,
    goal: Option<String>,
    installed_programs: bool,
) -> Result<()> {
    let links = match goal {
        Some(goal) => {
            let all_goals = ctx
                .project
                .goals
                .clone()
                .context("No goals set for project")?;
            let hash_map = ctx
                .project
                .links
                .clone()
                .into_iter()
                .map(|x| (x.name.clone(), x))
                .collect();
            all_goals
                .get(&goal)
                .context("Could not find goal")?
                .to_links(&hash_map, &all_goals)?
                .into_iter()
                .cloned()
                .collect()
        }
        None => {
            if installed_programs {
                let packages = ctx
                    .project
                    .programs
                    .as_ref()
                    .context("Could not find any packages")?;
                packages
                    .iter()
                    .map(|x| {
                        if x.package_installed()? {
                            x.get_goal(&ctx)
                        } else {
                            Ok(vec![])
                        }
                    })
                    .collect::<Result<Vec<_>, anyhow::Error>>()?
                    .concat()
            } else {
                ctx.project.links.clone()
            }
        }
    };

    link_links(ctx, links).await
}

pub async fn link_links(ctx: ProjectContext, links: Vec<Link>) -> Result<()> {
    let ctx = Arc::new(ctx);
    let threads = links.into_iter().map(|link| {
        let ctx = ctx.clone();
        //Create async threads to link
        tokio::spawn(async move {
            async {
                let project_path = &ctx.project_config_path;
                let source = match link.src.resolve(&ctx.system) {
                    Some(d) => project_path.join(d),
                    None => return Ok(()),
                }
                .canonicalize()?;

                //Normalise destination
                let destination = {
                    //Parse in environment variables
                    let mut temp_dest = link
                        .destination
                        .to_path_buf(ctx.project.variables.as_ref())?;
                    if temp_dest.is_dir()
                        && temp_dest.exists()
                        && !same_file::is_same_file(&temp_dest, &source)?
                    {
                        temp_dest.push(
                            source
                                .file_name()
                                .context(format!("Could not get file name for {}", link.name))?,
                        );
                    }
                    //If the destination exists, and links back to the original location, then already
                    //linked
                    if temp_dest.exists() && same_file::is_same_file(&temp_dest, &source)? {
                        info!(r#""{}" already linked"#, source.display());
                        return Ok(());
                    } else if temp_dest.exists() {
                        error!("{} file already exists", temp_dest.display());
                        return Ok(());
                    }
                    temp_dest
                };
                if link.sudo_required.unwrap_or(false) {
                    let sudo_program = ctx.system_config.sudo_program.as_deref().unwrap_or("sudo");
                    crate::util::elevate(
                        sudo_program,
                        &[
                            "mkdir",
                            "-p",
                            destination
                                .parent()
                                .and_then(|x| x.to_str())
                                .context("Could not get parent folder")?,
                        ],
                    )
                    .spawn()?
                    .wait()
                    .await?;
                    crate::util::elevate(
                        sudo_program,
                        &[
                            "ln",
                            "-s",
                            source
                                .to_str()
                                .context("Could not convert source to string")?,
                            destination
                                .to_str()
                                .context("Could not convert destination to string")?,
                        ],
                    )
                    .spawn()?
                    .wait()
                    .await?;
                } else {
                    fs::create_dir_all(
                        &destination
                            .parent()
                            .context("Could not get parent folder")?,
                    )
                    .await
                    .context(format!(
                        "Failed creating folder hierchy for {}",
                        &destination.display()
                    ))?;

                    fs::symlink(source, destination).await?;
                }
                Ok::<_, anyhow::Error>(())
            }
            .await
            .context(format!("Failed linking {}", &link.name))
        })
    });

    for res in threads {
        if let Err(e) = res.await.map_err(Into::into).flatten() {
            log::error!("Error syncing : {}", e)
        }
    }
    Ok(())
}
