use crate::{link::Link, ProjectContext};
use anyhow::{Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Goal {
    pub enabled: bool,
    pub links: Vec<String>,
    pub required_goals: Option<Vec<String>>,
}

impl Goal {
    pub fn new(required_goals: Vec<String>) -> Goal {
        Goal {
            enabled: true,
            links: Vec::new(),
            required_goals: if required_goals.is_empty() {
                None
            } else {
                Some(required_goals)
            },
        }
    }
    pub fn get_links(&self, ctx: &ProjectContext) -> Result<Vec<Link>> {
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
        Ok(self
            .to_links(&hash_map, &all_goals)?
            .into_iter()
            .cloned()
            .collect())
    }
    pub fn to_links<'a>(
        &self,
        all_links: &'a HashMap<String, Link>,
        all_goals: &'a HashMap<String, Goal>,
    ) -> Result<Vec<&'a Link>> {
        let mut links = self.links.clone();
        if let Some(ref x) = self.required_goals {
            links.extend(
                x.iter()
                    .map(|x| {
                        Ok(all_goals
                            .get(x)
                            .context(format!("Could not find {}", x))?
                            .links
                            .clone())
                    })
                    .try_collect::<_, Vec<_>, anyhow::Error>()?
                    .concat(),
            );
        }
        links
            .into_iter()
            .dedup()
            .map(|x| all_links.get(&x).context(format!("Could not find {}", &x)))
            .try_collect::<_, Vec<_>, _>()
    }
}

impl std::fmt::Display for Goal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Enabled: {}", self.enabled)?;
        writeln!(f, "  Links: ")?;
        for link in &self.links {
            writeln!(f, "{}", link)?;
        }
        Ok(())
    }
}
