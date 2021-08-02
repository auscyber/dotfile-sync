use crate::link::{Link, VariablePath};
use anyhow::{Context, Result};
use petgraph::Graph;
use serde::{Deserialize, Serialize};
use itertools::Itertools;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Goal {
    pub enabled: bool,
    pub name: String,
    pub links: Vec<String>,
    pub extern_script: Option<VariablePath>,
    pub local_script: Option<String>,
    pub required_goals: Option<Vec<String>>,
}

impl Goal {
    fn get_links(&self, ctx: &crate::ProjectContext) -> Result<Vec<Link>> {
        let project = &ctx.project;
        project.links.
        todo!();
    }
}
