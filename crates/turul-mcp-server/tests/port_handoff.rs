//! The ephemeral-port handoff used by the HTTP test suites must be exclusive.
//!
//! A reserved port is only free-and-unclaimed between the reservation listener
//! dropping and the server under test binding. If two tests are inside that
//! window at once the kernel can hand both the same port; the loser's readiness
//! probe then succeeds against the winner's server and its requests reach a
//! different handler set. `reserve_port` closes the window by admitting one
//! caller at a time, so the property under test is mutual exclusion.

mod common;

use std::time::Duration;

#[tokio::test]
async fn a_live_reservation_blocks_the_next_one() {
    let first = common::reserve_port().await;

    let blocked = tokio::time::timeout(Duration::from_millis(250), common::reserve_port()).await;
    assert!(
        blocked.is_err(),
        "a second reservation must not be handed out while the first is still \
         in its bind window (got port {:?})",
        blocked.map(|r| r.port).ok()
    );

    drop(first);

    let after = tokio::time::timeout(Duration::from_millis(2_000), common::reserve_port()).await;
    assert!(
        after.is_ok(),
        "dropping a reservation must release the handoff for the next caller"
    );
}

#[tokio::test]
async fn reservations_are_bindable_and_distinct_under_contention() {
    let mut handles = Vec::new();
    for _ in 0..16 {
        handles.push(tokio::spawn(async {
            let reserved = common::reserve_port().await;
            let port = reserved.port;
            // Stand in for the gap between reserving and the server binding.
            tokio::time::sleep(Duration::from_millis(5)).await;
            let bound = std::net::TcpListener::bind(("127.0.0.1", port))
                .unwrap_or_else(|e| panic!("port {port} was handed out twice: {e}"));
            drop(reserved);
            (port, bound)
        }));
    }

    let mut held = Vec::new();
    for h in handles {
        held.push(h.await.expect("reservation task"));
    }

    let mut ports: Vec<u16> = held.iter().map(|(p, _)| *p).collect();
    ports.sort_unstable();
    let distinct = ports.len();
    ports.dedup();
    assert_eq!(ports.len(), distinct, "two tasks were handed the same port");
}
