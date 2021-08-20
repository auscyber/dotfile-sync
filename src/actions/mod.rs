use crate::config::SystemConfig;
use anyhow::*;
use log::*;

mod add;
pub mod goal;
mod prune;
mod revert;
mod sync;

pub use add::add;
pub use prune::prune;
pub use revert::revert;
pub use sync::sync;

pub fn manage(ctx: &super::ProjectContext, make_default: bool) -> Result<SystemConfig> {
    let mut sysconfig = ctx.system_config.clone();
    if !ctx.project_config_path.exists() {
        bail!("Project path does not exist");
    }
    sysconfig.add_project(ctx.project.name.clone(), ctx.project_config_path.clone());
    if make_default {
        sysconfig.default = Some(ctx.project_config_path.clone());
        info!("Set as default");
    }

    Ok(sysconfig)
}
