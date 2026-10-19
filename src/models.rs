//! Data transferred between SQLite services and the terminal UI.
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,
    DistrictOfficial,
    Voter,
}
impl Role {
    /// Return the stable database role name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::DistrictOfficial => "district_official",
            Self::Voter => "voter",
        }
    }
    /// Return the role label shown in the TUI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Admin => "Administrator",
            Self::DistrictOfficial => "District Official",
            Self::Voter => "Voter",
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Candidate {
    pub id: i64,
    pub name: String,
    pub party: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub id: i64,
    pub name: String,
    pub candidates: Vec<Candidate>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Election {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub announced: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Voter {
    pub id: i64,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub dob: String,
    pub nid: String,
    pub has_voted: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VoterInput {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub dob: String,
    pub nid: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResultRow {
    pub position: String,
    pub candidate: String,
    pub party: String,
    pub votes: i64,
}
