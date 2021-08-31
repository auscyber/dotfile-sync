use crate::{goals::Goal, link::Link};
use anyhow::{Context, Result};
use std::env; use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum GoalType {
    InlineGoal(Goal),
    GoalName { goal: String },
    LinkName { link_name: String },
    Link(Link),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramConfig {
    app_name: String,
    app_aliases: Option<Vec<String>>,
    checker_script: Option<String>,
    #[serde(flatten)]
    goal: GoalType,
}

impl ProgramConfig {
    pub fn get_goal(&self, ctx: &crate::ProjectContext) -> Result<Vec<Link>> {
        match &self.goal {
            GoalType::InlineGoal(goal) => Ok(goal.get_links(ctx)?),
            GoalType::GoalName { goal: goal_name } => ctx
                .project
                .goals
                .as_ref()
                .context("no goals from project".to_string())?
                .get(goal_name)
                .context(format!("Goal could not be found {}", &goal_name))?
                .clone()
                .get_links(ctx),
            GoalType::LinkName { link_name } => Ok(ctx
                .project
                .links
                .iter()
                .find(|x| &x.name == link_name)
                .cloned()
                .into_iter()
                .collect()),
            GoalType::Link(link) => Ok(vec![link.clone()]),
        }
    }
    pub fn package_installed(&self) -> Result<bool> {
        Ok(if let Some(ref script) = self.checker_script {
            let result = std::process::Command::new("sh")
                .args(["-c", script])
                .spawn()?
                .wait()?;
            result.success()
        } else {
            false
        } || env::var("PATH")
            .context("Could not get PATH variable")?
            .split(';')
            .any(|x| {
                let path = PathBuf::from(x);
                match path.read_dir() {
                    Ok(mut dir) => dir
                        .find_map(|x| {
                            let file_name = x.ok()?.file_name();
                            let file_name = file_name.to_str()?;
                            Some(
                                file_name == self.app_name
                                    || self
                                        .app_aliases
                                        .as_ref()
                                        .and_then(|aliases| {
                                            aliases.iter().find(|y| file_name == *y)
                                        })
                                        .is_some(),
                            )
                        })
                        .is_some(),
                    _ => false,
                }
            }))
    }
}
