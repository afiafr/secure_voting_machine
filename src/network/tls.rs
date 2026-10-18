//! Loopback development PKI and bounded, length-prefixed JSON over TLS 1.3.
//! Public clients trust only the CA. Only gateways hold client certificates.
use super::*;
use rcgen::{
    BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use rustls::{
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName},
    RootCertStore,
};
use std::io::{Read, Write};

pub struct Identity {
    pub cert: Vec<u8>,
    pub key: Vec<u8>,
}
pub struct Certificates {
    pub ca: Vec<u8>,
    pub server: Identity,
    pub gateways: [Identity; 3],
}
impl Certificates {
    pub fn generate() -> AppResult<Self> {
        let ca_key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![])?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca = params.self_signed(&ca_key)?;
        let leaf = |client: bool| -> AppResult<Identity> {
            let key = KeyPair::generate()?;
            let mut p = CertificateParams::new(vec!["localhost".into()])?;
            p.extended_key_usages = vec![if client {
                ExtendedKeyUsagePurpose::ClientAuth
            } else {
                ExtendedKeyUsagePurpose::ServerAuth
            }];
            p.key_usages = vec![KeyUsagePurpose::DigitalSignature];
            let cert = p.signed_by(&key, &ca, &ca_key)?;
            Ok(Identity {
                cert: cert.der().to_vec(),
                key: key.serialize_der(),
            })
        };
        Ok(Self {
            ca: ca.der().to_vec(),
            server: leaf(false)?,
            gateways: [leaf(true)?, leaf(true)?, leaf(true)?],
        })
    }
    // The CA signing key is deliberately not persisted. Rotate the whole dev bundle.
    pub fn save(&self, path: &std::path::Path) -> AppResult<()> {
        std::fs::create_dir(path)?; // Refuse to overwrite an existing identity directory.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
        }
        let put = |name: &str, bytes: &[u8]| -> AppResult<()> {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(path.join(name))?.write_all(bytes)?;
            Ok(())
        };
        put("ca.der", &self.ca)?;
        for (name, id) in [
            ("server", &self.server),
            ("admin", &self.gateways[0]),
            ("district", &self.gateways[1]),
            ("voter", &self.gateways[2]),
        ] {
            put(&format!("{name}.der"), &id.cert)?;
            put(&format!("{name}.key"), &id.key)?;
        }
        Ok(())
    }
    pub fn load(path: &std::path::Path) -> AppResult<Self> {
        let get = |name: &str| -> AppResult<Identity> {
            Ok(Identity {
                cert: std::fs::read(path.join(format!("{name}.der")))?,
                key: std::fs::read(path.join(format!("{name}.key")))?,
            })
        };
        Ok(Self {
            ca: std::fs::read(path.join("ca.der"))?,
            server: get("server")?,
            gateways: [get("admin")?, get("district")?, get("voter")?],
        })
    }
    pub fn server_config(&self, mutual: bool) -> AppResult<Arc<rustls::ServerConfig>> {
        let builder =
            rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13]);
        let builder = if mutual {
            builder.with_client_cert_verifier(
                rustls::server::WebPkiClientVerifier::builder(Arc::new(roots(&self.ca)?))
                    .build()?,
            )
        } else {
            builder.with_no_client_auth()
        };
        Ok(Arc::new(builder.with_single_cert(
            vec![CertificateDer::from(self.server.cert.clone())],
            PrivatePkcs8KeyDer::from(self.server.key.clone()).into(),
        )?))
    }
}
fn roots(ca: &[u8]) -> AppResult<RootCertStore> {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(ca.to_vec()))?;
    Ok(roots)
}
pub fn client_config(
    ca: &[u8],
    identity: Option<&Identity>,
) -> AppResult<Arc<rustls::ClientConfig>> {
    let builder = rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_root_certificates(roots(ca)?);
    Ok(Arc::new(match identity {
        Some(id) => builder.with_client_auth_cert(
            vec![CertificateDer::from(id.cert.clone())],
            PrivatePkcs8KeyDer::from(id.key.clone()).into(),
        )?,
        None => builder.with_no_client_auth(),
    }))
}
pub fn socket(address: SocketAddr) -> AppResult<TcpStream> {
    let socket = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
    configure(&socket)?;
    Ok(socket)
}
pub fn configure(socket: &TcpStream) -> AppResult<()> {
    // Accepted sockets can inherit the listener's nonblocking mode on macOS/BSD.
    // Each TLS worker uses blocking I/O with deadlines, not an async readiness loop.
    socket.set_nonblocking(false)?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    socket.set_nodelay(true)?;
    Ok(())
}
/// A whole-request deadline also limits peers that drip bytes to reset idle timeouts.
pub struct DeadlineSocket {
    socket: TcpStream,
    deadline: Instant,
    idle_timeout: Duration,
}
impl DeadlineSocket {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            deadline: Instant::now() + Duration::from_secs(10),
            idle_timeout: Duration::from_secs(5),
        }
    }
    /// Password hashing can exceed the short request-read budget in debug builds.
    /// Keep a finite response budget, while retaining stricter inbound limits.
    pub fn begin_response(&mut self) {
        self.deadline = Instant::now() + Duration::from_secs(60);
        self.idle_timeout = Duration::from_secs(60);
    }
    fn remaining(&self) -> std::io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "TLS request deadline exceeded",
                )
            })
    }
}
impl Read for DeadlineSocket {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket
            .set_read_timeout(Some(self.remaining()?.min(self.idle_timeout)))?;
        self.socket.read(buf)
    }
}
impl Write for DeadlineSocket {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.socket
            .set_write_timeout(Some(self.remaining()?.min(self.idle_timeout)))?;
        self.socket.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }
}
const MAX_FRAME: usize = 1024 * 1024;
pub fn read_frame<T: serde::de::DeserializeOwned>(stream: &mut impl Read) -> AppResult<T> {
    let mut size = [0; 4];
    stream.read_exact(&mut size)?;
    let n = u32::from_be_bytes(size) as usize;
    if n > MAX_FRAME {
        return Err("Endpoint message exceeds 1 MiB.".into());
    }
    let mut bytes = vec![0; n];
    stream.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn write_frame<T: Serialize>(stream: &mut impl Write, value: &T) -> AppResult<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_FRAME {
        return Err("Endpoint message exceeds 1 MiB.".into());
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    Ok(())
}
pub fn exchange(
    address: SocketAddr,
    config: Arc<rustls::ClientConfig>,
    request: &Request,
) -> AppResult<Response> {
    exchange_named(address, config, request, "localhost")
}
pub(super) fn exchange_named(
    address: SocketAddr,
    config: Arc<rustls::ClientConfig>,
    request: &Request,
    name: &str,
) -> AppResult<Response> {
    let context = |stage: &str, error: Box<dyn std::error::Error>| -> Box<dyn std::error::Error> {
        format!("TLS endpoint {address}, {stage}: {error}").into()
    };
    let connection = rustls::ClientConnection::new(config, ServerName::try_from(name.to_owned())?)?;
    let socket = socket(address).map_err(|e| context("TCP connection", e))?;
    let mut stream = rustls::StreamOwned::new(connection, DeadlineSocket::new(socket));
    write_frame(&mut stream, request).map_err(|e| context("handshake/request send", e))?;
    // This protocol sends one request per connection. Close only our TLS write
    // side; TLS 1.3 still allows the peer to send its response afterward.
    stream.conn.send_close_notify();
    stream
        .flush()
        .map_err(|e| context("request shutdown", e.into()))?;
    stream.sock.begin_response();
    // Never treat a truncated length-prefixed response as a successful operation.
    read_frame(&mut stream).map_err(|e| context("response receive (no complete response)", e))
}
