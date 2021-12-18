use crate::{link::Link, ProjectContext};
use anyhow::{Context, Result};
use futures::TryStreamExt;
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
                let ctx = Arc::new(&ctx);
                packages
                    .iter()
                    .map(|x| async {
                        x.package_installed().await.and_then(|y| {
                            if y {
                                x.get_goal(&ctx)
                            } else {
                                Ok::<Vec<Link>, anyhow::Error>(vec![])
                            }
                        })
                    })
                    .collect::<futures::stream::FuturesUnordered<_>>()
                    .try_collect::<Vec<_>>()
                    .await?;
                todo!();
            } else {
                ctx.project.links.clone()
            }
        }
    };

    link_links(ctx, links).await
}

pub async fn link_links(ctx: ProjectContext, links: Vec<Link>) -> Result<()> {
    let ctx = Arc::new(ctx);
    let threads = links.into_iter().map(
        |link| -> tokio::task::JoinHandle<Result<(), anyhow::Error>> {
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
                    let destination =
                        {
                            //Parse in environment variables
                            let mut temp_dest = link
                                .destination
                                .to_path_buf(ctx.project.variables.as_ref())?;
                            if temp_dest.is_dir()
                                && temp_dest.exists()
                                && !same_file::is_same_file(&temp_dest, &source)?
                            {
                                temp_dest.push(source.file_name().context(format!(
                                    "Could not get file name for {}",
                                    link.name
                                ))?);
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
                    // If sudo is required to pass then set perms
                    if link.sudo_required.unwrap_or(false) {
                        let sudo_program =
                            ctx.system_config.sudo_program.as_deref().unwrap_or("sudo");
                        com_run(
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
                        .await?;
                        let dest_str = destination
                            .to_str()
                            .context("Could not convert destination to string")?;
                        com_run(
                            sudo_program,
                            &[
                                "ln",
                                "-s",
                                source
                                    .to_str()
                                    .context("Could not convert source to string")?,
                                dest_str,
                            ],
                        )
                        .await?;
                        if let Some(perms) = link.perms {
                            if perms.user_owner.is_some() || perms.group_owner.is_some() {
                                let owner_loc = format!(
                                    "{}:{}",
                                    perms.user_owner.unwrap_or_else(|| "".to_string()),
                                    perms.group_owner.unwrap_or_else(|| "".to_string())
                                );
                                com_run(sudo_program, &["chown", "-h", "-R", &owner_loc, dest_str])
                                    .await?;
                            }
                            if let Some(user_code) = perms.user_code {
                                com_run(sudo_program, &["chmod", "-R", &user_code, dest_str])
                                    .await?;
                            }
                            let source_parent = source
                                .parent()
                                .and_then(|x| x.to_str())
                                .context("Could not get destination parent")?;
                            log::debug!("dest_parent: {}", source_parent);
                            com_run(sudo_program, &["chmod", "o+rx", source_parent]).await?;
                        }
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

                        fs::symlink(source, &destination).await?;
                        if let Some(perms) = link.perms {
                            let dest_str = destination
                                .to_str()
                                .context("Could not convert destination to string")?;
                            if let Some(user_code) = perms.user_code {
                                com_run("chmod", &["-R", &user_code, dest_str]).await?;
                            }
                            if perms.user_owner.is_some() || perms.group_owner.is_some() {
                                let owner_str = format!(
                                    "{}:{}",
                                    perms.user_owner.unwrap_or_default(),
                                    perms.group_owner.unwrap_or_default()
                                );
                                com_run("chown", &["-R", &owner_str, dest_str]).await?;
                            }
                        }
                    }
                    Ok::<_, anyhow::Error>(())
                }
                .await
                .context(format!("Failed linking {}", &link.name))
            })
        },
    );

    for res in threads {
        if let Err(e) = res.await.map_err(Into::into).flatten() {
            log::error!("Error syncing : {}", e)
        }
    }
    Ok(())
}

async fn com_run<I, S>(com: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    crate::util::run_command(com, args).spawn()?.wait().await?;
    Ok(())
}
