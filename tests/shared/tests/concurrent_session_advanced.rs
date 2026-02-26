//! Advanced Concurrent Session Testing
//!
//! Tests complex multi-client scenarios including:
//! - High-concurrency client creation
//! - Resource contention and isolation
//! - Long-running operations with session persistence
//! - Cross-protocol session management
//! - Load testing with session cleanup

use mcp_e2e_shared::{McpTestClient, TestFixtures, TestServerManager};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Barrier, Mutex};
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Test high-concurrency client creation with immediate session validation
#[tokio::test]
async fn test_high_concurrency_client_creation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let client_count = 50;

    info!(
        "Starting high-concurrency test with {} clients",
        client_count
    );
    let start_time = Instant::now();

    // Create barrier for synchronized client creation
    let barrier = Arc::new(Barrier::new(client_count + 1));
    let session_ids = Arc::new(Mutex::new(Vec::new()));

    let mut handles = Vec::new();

    // Spawn concurrent client creation tasks
    for client_id in 0..client_count {
        let barrier_clone = barrier.clone();
        let session_ids_clone = session_ids.clone();
        let server_port = server.port();

        let handle = tokio::spawn(async move {
            let mut client = McpTestClient::new(server_port);

            // Wait for all clients to be ready before starting initialization
            barrier_clone.wait().await;

            let init_start = Instant::now();
            client
                .initialize()
                .await
                .expect("Failed to initialize client");
            let init_duration = init_start.elapsed();

            let session_id = client.session_id().unwrap().clone();

            // Collect session ID for uniqueness verification
            {
                let mut sessions = session_ids_clone.lock().await;
                sessions.push(session_id.clone());
            }

            // Perform a quick operation to verify session works
            let _result = client.list_resources().await;

            (client_id, session_id, init_duration)
        });

        handles.push(handle);
    }

    // Start all clients simultaneously
    barrier.wait().await;
    info!("All {} clients started simultaneously", client_count);

    // Wait for all clients to complete
    let results = futures::future::join_all(handles).await;
    let total_duration = start_time.elapsed();

    // Verify all clients succeeded
    let mut session_set = HashSet::new();
    let mut init_times = Vec::new();

    for result in results {
        let (client_id, session_id, init_duration) = result.expect("Client task should succeed");

        // Verify session uniqueness
        assert!(
            session_set.insert(session_id.clone()),
            "Session ID {} for client {} should be unique",
            session_id,
            client_id
        );

        init_times.push(init_duration);
        debug!(
            "Client {} completed with session {} in {:?}",
            client_id, session_id, init_duration
        );
    }

    // Calculate performance metrics
    let avg_init_time = init_times.iter().sum::<Duration>() / init_times.len() as u32;
    let max_init_time = init_times.iter().max().unwrap();
    let min_init_time = init_times.iter().min().unwrap();

    info!("✅ High-concurrency test completed:");
    info!("  Total clients: {}", client_count);
    info!("  Total time: {:?}", total_duration);
    info!("  Average init time: {:?}", avg_init_time);
    info!(
        "  Min/Max init time: {:?} / {:?}",
        min_init_time, max_init_time
    );
    info!(
        "  All session IDs unique: {}",
        session_set.len() == client_count
    );

    // Performance assertions
    assert!(
        avg_init_time < Duration::from_secs(5),
        "Average init time should be reasonable"
    );
    assert!(
        session_set.len() == client_count,
        "All sessions should be unique"
    );
}

/// Test resource contention scenarios with multiple clients accessing same resources
#[tokio::test]
async fn test_resource_contention_isolation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let client_count = 20;

    info!("Testing resource contention with {} clients", client_count);

    // Create clients
    let mut clients = Vec::new();
    for _i in 0..client_count {
        let mut client = McpTestClient::new(server.port());
        client
            .initialize()
            .await
            .expect("Failed to initialize client");
        clients.push(client);
    }

    // Verify all have unique sessions
    let session_ids: Vec<String> = clients
        .iter()
        .map(|c| c.session_id().unwrap().clone())
        .collect();
    let unique_sessions: HashSet<_> = session_ids.iter().collect();
    assert_eq!(
        unique_sessions.len(),
        client_count,
        "All sessions should be unique"
    );

    // Test concurrent access to the same resources
    let test_resources = vec![
        "file:///memory/data.json",
        "file:///session/info.json",
        "file:///tmp/test.txt",
        "file:///empty/content.txt",
    ];

    let results = Arc::new(Mutex::new(Vec::<(
        usize,
        String,
        Vec<(String, usize, bool, Option<String>)>,
    )>::new()));
    let mut handles = Vec::new();

    for (client_id, client) in clients.into_iter().enumerate() {
        let resources = test_resources.clone();
        let _results_clone = results.clone();

        let handle = tokio::spawn(async move {
            let session_id = client.session_id().unwrap().clone();
            let mut client_results = Vec::new();

            // Access each resource multiple times
            for resource_uri in resources {
                for attempt in 0..3 {
                    let result = client.read_resource(resource_uri).await;

                    match result {
                        Ok(response) => {
                            client_results.push((resource_uri.to_string(), attempt, true, None));

                            // Verify session-aware resources include correct session
                            if resource_uri == "file:///session/info.json"
                                && let Some(result_data) =
                                    response.get("result").and_then(|r| r.as_object())
                                && let Some(contents) =
                                    result_data.get("contents").and_then(|c| c.as_array())
                                && let Some(content) = contents
                                    .first()
                                    .and_then(|item| item.get("text"))
                                    .and_then(|t| t.as_str())
                                && !content.contains("session")
                            {
                                warn!(
                                    "Session resource missing session info for client {}",
                                    client_id
                                );
                            }
                        }
                        Err(e) => {
                            client_results.push((
                                resource_uri.to_string(),
                                attempt,
                                false,
                                Some(e.to_string()),
                            ));
                        }
                    }

                    // Small delay between attempts
                    sleep(Duration::from_millis(10)).await;
                }
            }

            (client_id, session_id, client_results)
        });

        handles.push(handle);
    }

    // Wait for all concurrent access
    let client_results = futures::future::join_all(handles).await;

    // Analyze results
    let mut total_operations = 0;
    let mut successful_operations = 0;
    let mut session_errors = 0;

    for result in client_results {
        let (client_id, session_id, operations) =
            result.expect("Client operations should complete");

        for (resource, _attempt, success, error) in operations {
            total_operations += 1;

            if success {
                successful_operations += 1;
            } else if let Some(err) = error
                && err.to_lowercase().contains("session")
            {
                session_errors += 1;
                warn!(
                    "Session error for client {} on {}: {}",
                    client_id, resource, err
                );
            }
        }

        debug!(
            "Client {} ({}) completed all resource access operations",
            client_id, session_id
        );
    }

    let success_rate = (successful_operations as f64 / total_operations as f64) * 100.0;

    info!("✅ Resource contention test completed:");
    info!("  Total operations: {}", total_operations);
    info!("  Successful operations: {}", successful_operations);
    info!("  Success rate: {:.1}%", success_rate);
    info!("  Session errors: {}", session_errors);

    // Should have high success rate and no session-related errors
    assert!(
        success_rate > 70.0,
        "Should have reasonable success rate under contention"
    );
    assert_eq!(session_errors, 0, "Should not have session-related errors");
}

/// Test long-running operations with session persistence
#[tokio::test]
async fn test_long_running_session_persistence() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_prompts_server()
        .await
        .expect("Failed to start server");
    let client_count = 10;
    let operation_duration = Duration::from_secs(2);

    info!(
        "Testing long-running operations with {} clients for {:?}",
        client_count, operation_duration
    );

    let mut handles = Vec::new();

    for client_id in 0..client_count {
        let server_port = server.port();
        let duration = operation_duration;

        let handle = tokio::spawn(async move {
            let mut client = McpTestClient::new(server_port);
            client
                .initialize()
                .await
                .expect("Failed to initialize client");

            let initial_session = client.session_id().unwrap().clone();
            let start_time = Instant::now();
            let mut operation_count = 0;

            // Perform operations for the specified duration
            while start_time.elapsed() < duration {
                // Alternate between different operations
                let operation_result = match operation_count % 3 {
                    0 => client.list_prompts().await,
                    1 => client.get_prompt("session_aware_prompt", None).await,
                    _ => {
                        let mut args = HashMap::new();
                        args.insert(
                            "required_text".to_string(),
                            Value::String("test code".to_string()),
                        );
                        args.insert(
                            "optional_text".to_string(),
                            Value::String("test analysis".to_string()),
                        );
                        client.get_prompt("string_args_prompt", Some(args)).await
                    }
                };

                match operation_result {
                    Ok(_) => {
                        // Verify session hasn't changed
                        assert_eq!(
                            client.session_id().unwrap(),
                            &initial_session,
                            "Session should remain consistent during long operations"
                        );
                        operation_count += 1;
                    }
                    Err(e) => {
                        if e.to_string().to_lowercase().contains("session") {
                            panic!(
                                "Session error during long operation for client {}: {}",
                                client_id, e
                            );
                        }
                        // Other errors are acceptable for this test
                    }
                }

                // Small delay between operations
                sleep(Duration::from_millis(100)).await;
            }

            let final_session = client.session_id().unwrap().clone();
            let total_duration = start_time.elapsed();

            (
                client_id,
                initial_session,
                final_session,
                operation_count,
                total_duration,
            )
        });

        handles.push(handle);
    }

    // Wait for all long-running operations
    let results = futures::future::join_all(handles).await;

    // Verify all sessions remained consistent
    let mut total_operations = 0;

    for result in results {
        let (client_id, initial_session, final_session, operation_count, duration) =
            result.expect("Long-running operation should complete");

        assert_eq!(
            initial_session, final_session,
            "Client {} session should remain consistent: {} -> {}",
            client_id, initial_session, final_session
        );

        total_operations += operation_count;

        info!(
            "Client {} completed {} operations in {:?} (session: {})",
            client_id, operation_count, duration, final_session
        );
    }

    info!("✅ Long-running session persistence test completed:");
    info!(
        "  Total operations across all clients: {}",
        total_operations
    );
    info!("  All sessions remained consistent throughout operations");

    assert!(
        total_operations > 0,
        "Should have performed some operations"
    );
}

/// Test cross-protocol session management (resources + prompts)
#[tokio::test]
async fn test_cross_protocol_session_management() {
    let _ = tracing_subscriber::fmt::try_init();

    let resource_server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start resource server");
    let prompts_server = TestServerManager::start_prompts_server()
        .await
        .expect("Failed to start prompts server");
    let client_pairs = 15;

    info!(
        "Testing cross-protocol session management with {} client pairs",
        client_pairs
    );

    let mut handles = Vec::new();

    for pair_id in 0..client_pairs {
        let resource_port = resource_server.port();
        let prompts_port = prompts_server.port();

        let handle = tokio::spawn(async move {
            // Create clients for both protocols
            let mut resource_client = McpTestClient::new(resource_port);
            let mut prompts_client = McpTestClient::new(prompts_port);

            // Initialize both clients
            resource_client
                .initialize_with_capabilities(TestFixtures::resource_capabilities())
                .await
                .expect("Failed to initialize resource client");
            prompts_client
                .initialize_with_capabilities(TestFixtures::prompts_capabilities())
                .await
                .expect("Failed to initialize prompts client");

            let resource_session = resource_client.session_id().unwrap().clone();
            let prompts_session = prompts_client.session_id().unwrap().clone();

            // Sessions should be different between protocols
            assert_ne!(
                resource_session, prompts_session,
                "Different protocol sessions should be unique for pair {}",
                pair_id
            );

            // Perform interleaved operations on both protocols
            let mut operations_completed = 0;

            for _op in 0..10 {
                // Resource operation
                let resource_result = resource_client.list_resources().await;
                if resource_result.is_ok() {
                    operations_completed += 1;

                    // Verify session consistency
                    assert_eq!(
                        resource_client.session_id().unwrap(),
                        &resource_session,
                        "Resource session should remain consistent"
                    );
                }

                // Prompts operation
                let prompts_result = prompts_client.list_prompts().await;
                if prompts_result.is_ok() {
                    operations_completed += 1;

                    // Verify session consistency
                    assert_eq!(
                        prompts_client.session_id().unwrap(),
                        &prompts_session,
                        "Prompts session should remain consistent"
                    );
                }

                // Small delay between operations
                sleep(Duration::from_millis(50)).await;
            }

            (
                pair_id,
                resource_session,
                prompts_session,
                operations_completed,
            )
        });

        handles.push(handle);
    }

    // Wait for all cross-protocol operations
    let results = futures::future::join_all(handles).await;

    // Verify results
    let mut all_resource_sessions = HashSet::new();
    let mut all_prompts_sessions = HashSet::new();
    let mut total_operations = 0;

    for result in results {
        let (pair_id, resource_session, prompts_session, operations) =
            result.expect("Cross-protocol operations should complete");

        // Collect sessions for uniqueness verification
        assert!(
            all_resource_sessions.insert(resource_session.clone()),
            "Resource session {} for pair {} should be unique",
            resource_session,
            pair_id
        );
        assert!(
            all_prompts_sessions.insert(prompts_session.clone()),
            "Prompts session {} for pair {} should be unique",
            prompts_session,
            pair_id
        );

        total_operations += operations;

        debug!(
            "Pair {} completed {} operations (Resource: {}, Prompts: {})",
            pair_id, operations, resource_session, prompts_session
        );
    }

    info!("✅ Cross-protocol session management test completed:");
    info!("  Client pairs: {}", client_pairs);
    info!(
        "  Unique resource sessions: {}",
        all_resource_sessions.len()
    );
    info!("  Unique prompts sessions: {}", all_prompts_sessions.len());
    info!("  Total operations: {}", total_operations);

    // All sessions should be unique within each protocol
    assert_eq!(
        all_resource_sessions.len(),
        client_pairs,
        "All resource sessions should be unique"
    );
    assert_eq!(
        all_prompts_sessions.len(),
        client_pairs,
        "All prompts sessions should be unique"
    );
    assert!(
        total_operations > 0,
        "Should have completed some operations"
    );
}

/// Test session cleanup and memory management under load
#[tokio::test]
async fn test_session_cleanup_under_load() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server()
        .await
        .expect("Failed to start server");
    let waves = 5;
    let clients_per_wave = 20;
    let wave_interval = Duration::from_secs(1);

    info!(
        "Testing session cleanup with {} waves of {} clients",
        waves, clients_per_wave
    );

    let all_session_ids = Arc::new(Mutex::new(Vec::new()));

    for wave in 0..waves {
        info!(
            "Starting wave {} with {} clients",
            wave + 1,
            clients_per_wave
        );

        let mut wave_handles = Vec::new();

        for client_id in 0..clients_per_wave {
            let server_port = server.port();
            let session_ids_clone = all_session_ids.clone();

            let handle = tokio::spawn(async move {
                let mut client = McpTestClient::new(server_port);
                client
                    .initialize()
                    .await
                    .expect("Failed to initialize client");

                let session_id = client.session_id().unwrap().clone();

                // Record session ID
                {
                    let mut sessions = session_ids_clone.lock().await;
                    sessions.push(session_id.clone());
                }

                // Perform a few operations
                for _op in 0..3 {
                    let _result = client.list_resources().await;
                    sleep(Duration::from_millis(100)).await;
                }

                // Let client drop (simulating disconnect)
                drop(client);

                (wave, client_id, session_id)
            });

            wave_handles.push(handle);
        }

        // Wait for this wave to complete
        let wave_results = futures::future::join_all(wave_handles).await;

        for result in wave_results {
            let (wave_num, client_id, session_id) = result.expect("Wave client should complete");
            debug!(
                "Wave {} client {} completed with session {}",
                wave_num, client_id, session_id
            );
        }

        // Wait before next wave
        if wave < waves - 1 {
            sleep(wave_interval).await;
        }
    }

    // Verify session ID uniqueness across all waves
    let session_ids = all_session_ids.lock().await;
    let unique_sessions: HashSet<_> = session_ids.iter().collect();
    let total_expected = waves * clients_per_wave;

    info!("✅ Session cleanup test completed:");
    info!("  Total sessions created: {}", session_ids.len());
    info!("  Unique sessions: {}", unique_sessions.len());
    info!("  Expected sessions: {}", total_expected);

    assert_eq!(
        session_ids.len(),
        total_expected,
        "Should have created expected number of sessions"
    );
    assert_eq!(
        unique_sessions.len(),
        total_expected,
        "All sessions should be unique"
    );

    // Note: Actual cleanup verification would require server-side metrics
    // This test primarily verifies that session creation scales and remains unique under load
}
