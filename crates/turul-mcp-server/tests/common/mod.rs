//! Shared support for the 2026-07-28 server suites.

use tokio::sync::{Mutex, MutexGuard};

/// Held for the whole reserve-a-free-port → server-has-bound-it window.
///
/// `TcpListener::bind("127.0.0.1:0")` only reports which port was free at that
/// instant; the port is free again the moment that listener drops, which is
/// before the server under test binds it. Two tests in this binary whose
/// windows overlap can be handed the same port, and the loser's readiness probe
/// then succeeds against the winner's server — its requests reach a different
/// handler set. Serialising the window leaves at most one test inside it, and
/// once the server has bound the port the kernel will not hand it out again.
static PORT_HANDOFF: Mutex<()> = Mutex::const_new(());

/// A free ephemeral port plus the handoff lock. Keep this value alive until the
/// server has actually bound `port`; dropping it releases the lock.
pub struct ReservedPort {
    pub port: u16,
    _guard: MutexGuard<'static, ()>,
}

pub async fn reserve_port() -> ReservedPort {
    let guard = PORT_HANDOFF.lock().await;
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("reserve an ephemeral port")
        .local_addr()
        .expect("local_addr of the reservation listener")
        .port();
    ReservedPort {
        port,
        _guard: guard,
    }
}
