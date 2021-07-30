use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Goal {
    pub enabled: bool,
    pub name: String,
    pub required_goals: Option<Vec<String>>,
}
