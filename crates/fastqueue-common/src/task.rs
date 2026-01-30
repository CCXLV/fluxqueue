use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub args: Vec<u8>,
    pub kwargs: Vec<u8>,
    pub created_at: u64,
    pub retries: u8,
    pub max_retries: u8,
}
