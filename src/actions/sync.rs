use crate::{config::*, link::*};
use anyhow::{Context, Result};
use log::*;
use std::{fs, path::PathBuf, sync::Arc};

use crate::ProjectContext;
use cascade;

use futures::future::join_all;
pub async fn sync(ctx: ProjectContext) -> Result<()> {
    let ctx = Arc::new(ctx);
    let threads = ctx.project.links.clone().into_iter().map(|link| {
        let ctx = ctx.clone();
        tokio::spawn(async move {
            let project_path = &ctx.project_config_path;
            let destination = match link.src.resolve(&ctx.system) {
                Some(d) => project_path.join(d),
                None => return Ok(()),
            };
            debug!(
                "project_path is {}",
                project_path.join(&destination).display()
            );
            info!("Linking {}", link.name);
            let src = link.destination.to_path_buf(None)?;
            crate::util::create_folders(&src).context(format!(
                "Failed creating folder hierchy for {}",
                &src.display()
            ))?;

            if let Err(err) = fs::soft_link(project_path.join(destination), src)
                .context(format!("Failed linking {}", &link.name))
            {
                error!("{}", err);
                err.chain()
                    .skip(1)
                    .for_each(|cause| error!("\treason : {}", cause));
            };
            Ok::<_, anyhow::Error>(())
        })
    });
    join_all(threads)
        .await
        .into_iter()
        .map(|x| x?)
        .collect::<Result<Vec<_>>>()?;
    Ok(())
}
