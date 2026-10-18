//! Aggregate rejected transports so floods cannot create one database write per packet.
use super::*;
pub(super) fn start(
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    rejected: Arc<AtomicUsize>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut failures = 0;
        while !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
            let count = rejected.swap(0, Ordering::Relaxed);
            if count == 0 {
                continue;
            }
            failures += count;
            if let Ok(state) = state.lock() {
                let (event, level) = if failures > 10 {
                    failures = 0;
                    ("tls_or_protocol_flood", 2)
                } else {
                    ("tls_or_protocol_rejected", 3)
                };
                let _ = services::audit(&state.conn, None, None, None, event, level);
                let _ = state.auth.flush_notifications(&state.conn);
            }
        }
    })
}
