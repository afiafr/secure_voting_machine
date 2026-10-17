//! Client transport, authentication, and typed domain endpoint calls.
use super::*;
pub mod election;
pub mod voters;
pub struct ClientAuth;
#[derive(Clone)]
pub struct ApiClient {
    pub(super) addresses: [SocketAddr; 3],
    pub(super) config: Arc<rustls::ClientConfig>,
    pub(super) setup_token: Option<String>,
}
impl ApiClient {
    pub fn connect(ca: &[u8], base_port: u16) -> AppResult<Self> {
        let addresses =
            std::array::from_fn(|i| SocketAddr::from(([127, 0, 0, 1], base_port + 1 + i as u16)));
        Ok(Self {
            addresses,
            config: tls::client_config(ca, None)?,
            setup_token: None,
        })
    }
    pub(super) fn request<T: serde::de::DeserializeOwned>(
        &self,
        role: Role,
        token: Option<String>,
        operation: &str,
        args: Value,
    ) -> AppResult<T> {
        let response = tls::exchange(
            self.addresses[role_index(role)],
            self.config.clone(),
            &Request {
                role,
                token,
                operation: operation.into(),
                args,
            },
        )
        .map_err(|error| -> Box<dyn std::error::Error> {
            format!("{} gateway transport failed: {error}", role.label()).into()
        })?;
        if let Some(error) = response.error {
            return Err(error.into());
        }
        Ok(serde_json::from_value(response.value)?)
    }
    pub(super) fn call<T: serde::de::DeserializeOwned>(
        &self,
        session: &Session,
        operation: &str,
        args: Value,
    ) -> AppResult<T> {
        self.request(session.role, Some(session.token.clone()), operation, args)
    }
    pub fn logout(&self, session: &Session) -> AppResult<()> {
        self.call(session, "logout", Value::Null)
    }
    pub fn report_error(
        &self,
        session: Option<&Session>,
        screen: &str,
        message: &str,
    ) -> AppResult<()> {
        // Authentication failures are recorded by the server already.
        if let Some(s) = session {
            self.call(s, "error", json!([screen, message]))
        } else {
            Ok(())
        }
    }
}
impl ClientAuth {
    pub fn password_login(
        &self,
        conn: &ApiClient,
        email: &str,
        password: &str,
        role: Role,
    ) -> AppResult<PendingLogin> {
        conn.request(role, None, "password", json!([email, password]))
    }
    pub fn pin_login(&self, conn: &ApiClient, pin: &str) -> AppResult<PendingLogin> {
        conn.request(Role::Voter, None, "pin", json!([pin]))
    }
    pub fn complete_otp(
        &self,
        conn: &ApiClient,
        pending: &mut Option<PendingLogin>,
        code: &str,
    ) -> AppResult<Session> {
        let p = pending.as_ref().ok_or("Start login again.")?;
        let result = conn.request(p.role, Some(p.token.clone()), "otp", json!([code]));
        if result.is_ok() {
            *pending = None;
        }
        result
    }
}

pub fn provision(
    conn: &mut ApiClient,
    _auth: &ClientAuth,
    email: &str,
    phone: &str,
    name: &str,
    password: &str,
    role: Role,
) -> AppResult<()> {
    conn.request(
        Role::Admin,
        conn.setup_token.clone(),
        "provision",
        json!([email, phone, name, password, role]),
    )
}
