//! Wire records. Session tokens are opaque credentials, not caller-selected identities.
use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default)]
    pub(super) deadline_ms: u64,
    pub(super) role: Role,
    pub(super) token: Option<String>,
    pub(super) operation: String,
    pub(super) args: Value,
}
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub(super) value: Value,
    pub(super) error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub(super) token: String,
    pub(super) user_id: i64,
    pub(super) role: Role,
}
impl Session {
    /// Return the authenticated user ID.
    pub fn user_id(&self) -> i64 {
        self.user_id
    }
    /// Return the authenticated role.
    pub fn role(&self) -> Role {
        self.role
    }
}
#[derive(Serialize, Deserialize)]
pub struct PendingLogin {
    pub(super) token: String,
    pub(super) role: Role,
}
