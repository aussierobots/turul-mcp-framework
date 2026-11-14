//! Shared support for the 2026-07-28 server suites.

/// Bind an ephemeral loopback port and return the listener with its port.
/// Hand the listener to the server builder (`.listener(listener)`) so the port
/// is never released between choosing it and the server accepting on it.
pub fn bind_ephemeral() -> (std::net::TcpListener, u16) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
    let port = listener.local_addr().expect("local_addr").port();
    (listener, port)
}
