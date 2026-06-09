use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disciple {
    pub id: String,
    pub name: String,
    pub talent: i32,
    pub inner_power: i32,
    pub martial_art: String,
    pub loyalty: i32,
    pub months_in_sect: i32,
    pub alive: bool,
}
