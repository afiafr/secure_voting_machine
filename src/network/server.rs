//! TLS listeners, gateway forwarding, and worker lifecycle.
use super::*;
use std::io::Write;
/// Owns worker lifetimes. The default classroom mode launches all four listeners
/// in one process, but every UI operation still crosses both real TLS connections.
pub struct Network {
    pub(super) stop: Arc<AtomicBool>,
    pub(super) workers: Vec<thread::JoinHandle<()>>,
    pub client: ApiClient,
    #[cfg(test)]
    pub(super) state: Arc<Mutex<State>>,
    #[cfg(test)]
    pub(super) db_address: SocketAddr,
}
impl Network {
    pub fn start(
        conn: Connection,
        auth: auth::login::Auth,
        certs: tls::Certificates,
        base_port: Option<u16>,
        setup: bool,
    ) -> AppResult<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let setup_token = setup.then(|| Uuid::new_v4().to_string());
        let state = Arc::new(Mutex::new(State {
            conn,
            auth,
            pending: HashMap::new(),
            sessions: HashMap::new(),
            setup_token: setup_token.clone(),
        }));
        let bind = |offset: u16| -> AppResult<TcpListener> {
            Ok(TcpListener::bind(SocketAddr::from((
                [127, 0, 0, 1],
                base_port.map_or(0, |p| p + offset),
            )))?)
        };
        let db = bind(0)?;
        let db_address = db.local_addr()?;
        let gateways = [bind(1)?, bind(2)?, bind(3)?];
        let db_config = certs.server_config(true)?;
        let public_config = certs.server_config(false)?;
        let internal_configs = [
            tls::client_config(&certs.ca, Some(&certs.gateways[0]))?,
            tls::client_config(&certs.ca, Some(&certs.gateways[1]))?,
            tls::client_config(&certs.ca, Some(&certs.gateways[2]))?,
        ];
        let public_client = tls::client_config(&certs.ca, None)?;
        let rejected = Arc::new(AtomicUsize::new(0));
        let certificates = certs
            .gateways
            .iter()
            .map(|g| g.cert.clone())
            .collect::<Vec<_>>();
        let db_state = state.clone();
        let db_handler = move |request: Request, peer: Option<Vec<u8>>| -> Response {
            let result = (|| {
                let index = certificates
                    .iter()
                    .position(|c| Some(c) == peer.as_ref())
                    .ok_or("Unrecognized gateway certificate.")?;
                let mut state = db_state
                    .lock()
                    .map_err(|_| "Database service unavailable.")?;
                let result = state.dispatch(request, ROLES[index]);
                // Delivery failures leave an outbox entry for the next request.
                let _ = state.auth.flush_notifications(&state.conn);
                result
            })();
            response(result)
        };
        let mut workers = vec![listen(
            db,
            db_config,
            stop.clone(),
            rejected.clone(),
            db_handler,
        )?];
        let mut addresses = Vec::new();
        for (i, listener) in gateways.into_iter().enumerate() {
            let role = ROLES[i];
            addresses.push(listener.local_addr()?);
            let internal = internal_configs[i].clone();
            workers.push(listen(
                listener,
                public_config.clone(),
                stop.clone(),
                rejected.clone(),
                move |request, _| {
                    if request.role != role {
                        return response(Err("Wrong role gateway.".into()));
                    }
                    match tls::exchange(db_address, internal.clone(), &request) {
                        Ok(reply) => reply,
                        Err(error) => response(Err(format!(
                            "Database service transport failed: {error}"
                        )
                        .into())),
                    }
                },
            )?);
        }
        let client = ApiClient {
            addresses: addresses.try_into().expect("three role listeners"),
            config: public_client,
            setup_token,
        };
        workers.push(super::monitor::start(state.clone(), stop.clone(), rejected));
        Ok(Self {
            stop,
            workers,
            client,
            #[cfg(test)]
            state,
            #[cfg(test)]
            db_address,
        })
    }
    #[cfg(test)]
    pub fn test_session(&self, id: i64, role: Role) -> Session {
        let token = Uuid::new_v4().to_string();
        self.state.lock().unwrap().sessions.insert(
            token.clone(),
            SessionRecord {
                session: auth::login::Session::for_test(id, role),
                expires: Instant::now() + Duration::from_secs(1800),
            },
        );
        Session {
            token,
            user_id: id,
            role,
        }
    }
    #[cfg(test)]
    pub fn count(&self, table: &str) -> i64 {
        self.state
            .lock()
            .unwrap()
            .conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
}
impl Drop for Network {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
fn response(result: AppResult<Value>) -> Response {
    match result {
        Ok(value) => Response { value, error: None },
        Err(e) => Response {
            value: Value::Null,
            error: Some(e.to_string()),
        },
    }
}
fn listen(
    listener: TcpListener,
    config: Arc<rustls::ServerConfig>,
    stop: Arc<AtomicBool>,
    rejected: Arc<AtomicUsize>,
    handler: impl Fn(Request, Option<Vec<u8>>) -> Response + Send + Sync + 'static,
) -> AppResult<thread::JoinHandle<()>> {
    listener.set_nonblocking(true)?;
    let handler = Arc::new(handler);
    Ok(thread::spawn(move || {
        let active = Arc::new(AtomicUsize::new(0));
        let mut children = Vec::new();
        while !stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((socket, _)) => {
                    if active.load(Ordering::Relaxed) >= 32 {
                        rejected.fetch_add(1, Ordering::Relaxed);
                        drop(socket);
                        continue;
                    }
                    active.fetch_add(1, Ordering::Relaxed);
                    let (config, handler, active) =
                        (config.clone(), handler.clone(), active.clone());
                    let rejected = rejected.clone();
                    children.push(thread::spawn(move || {
                        let result: AppResult<()> = (|| {
                            tls::configure(&socket)?;
                            let mut stream = rustls::StreamOwned::new(
                                rustls::ServerConnection::new(config)?,
                                tls::DeadlineSocket::new(socket),
                            );
                            let request: Request = tls::read_frame(&mut stream)?;
                            // The request is complete. Allow bounded time for bcrypt and
                            // database work without relaxing slow-client request limits.
                            stream.sock.begin_response();
                            let peer = stream
                                .conn
                                .peer_certificates()
                                .and_then(|c| c.first())
                                .map(|c| c.to_vec());
                            let reply = handler(request, peer);
                            tls::write_frame(&mut stream, &reply)?;
                            // StreamOwned does not send TLS close_notify when dropped.
                            stream.conn.send_close_notify();
                            stream.flush()?;
                            Ok(())
                        })();
                        // Log counts, never untrusted request bodies or credentials.
                        if result.is_err() {
                            rejected.fetch_add(1, Ordering::Relaxed);
                        }
                        active.fetch_sub(1, Ordering::Relaxed);
                    }));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(_) => break,
            }
            let mut i = 0;
            while i < children.len() {
                if children[i].is_finished() {
                    let _ = children.swap_remove(i).join();
                } else {
                    i += 1;
                }
            }
        }
        for child in children {
            let _ = child.join();
        }
    }))
}
