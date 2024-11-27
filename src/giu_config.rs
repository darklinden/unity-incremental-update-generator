use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GIUConfig {
    pub unity_path: String,
    pub platforms: Vec<String>,
}
