//! # Lab 01: Rust Fundamentals for Go Developers & Cloud Architects
//!
//! Run this lab with:
//! ```bash
//! cargo run -p lab-01-rust-basics
//! ```

use std::sync::Arc;
use std::thread;

use lb_core::{
    validate_local_listener_addr, Backend, HealthStatus, LbError, RoundRobinSelector, UpstreamPool,
};

fn main() -> Result<(), LbError> {
    println!("==================================================================");
    println!("  Lab 01: Base Rust for Go Developers -> Load Balancer Primitives ");
    println!("==================================================================\n");

    // -------------------------------------------------------------------------
    // 1. Error Handling with Result<T, E> and the `?` operator
    // -------------------------------------------------------------------------
    // Go equivalent:
    //   listenAddr, err := validateLocalListenerAddr("127.0.0.1:6188")
    //   if err != nil { return err }
    let listen_addr = validate_local_listener_addr("127.0.0.1:6188")?;
    println!("[1] Validated local proxy listener address: {listen_addr}");

    // Demonstrate explicit error matching (like `errors.Is` / type switch in Go,
    // but checked exhaustively by the compiler):
    match validate_local_listener_addr("0.0.0.0:6188") {
        Ok(_) => unreachable!("wildcard bind should be rejected"),
        Err(err) => println!("    Security check blocked wildcard bind as expected: {err}\n"),
    }

    // -------------------------------------------------------------------------
    // 2. Ownership, Vec<T>, and Rich Enums (Sum Types)
    // -------------------------------------------------------------------------
    let backends = vec![
        Backend::new("127.0.0.1:8081", 1)?,
        Backend::new("127.0.0.1:8082", 1)?.with_status(HealthStatus::Unhealthy {
            consecutive_failures: 5,
        }),
        Backend::new("127.0.0.1:8083", 2)?.with_status(HealthStatus::Degraded {
            reason: "p99 latency > 250ms".to_string(),
        }),
        Backend::new("127.0.0.1:8084", 1)?,
    ];

    println!("[2] Configured Upstream Backends (Borrowing slice `&[Backend]`):");
    inspect_backends(&backends);
    println!();

    // -------------------------------------------------------------------------
    // 3. Traits, Generics & Move Semantics
    // -------------------------------------------------------------------------
    // Notice `backends` is MOVED into `UpstreamPool::new`. After this line,
    // `backends` cannot be used directly anymore — `pool` now owns that memory!
    let pool = UpstreamPool::new("payments-service", backends, RoundRobinSelector::new());
    println!(
        "[3] Created UpstreamPool '{}' using '{}' strategy",
        pool.name(),
        pool.strategy_name()
    );

    // -------------------------------------------------------------------------
    // 4. Fearless Concurrency: Arc<T> across Worker Threads
    // -------------------------------------------------------------------------
    // In Go, garbage collection lets any goroutine hold a pointer `*UpstreamPool`.
    // In Rust (without GC), `Arc` (Atomically Reference Counted smart pointer)
    // provides shared ownership across threads. Because `UpstreamPool` only uses
    // thread-safe types (`AtomicUsize`), the compiler automatically proves it is
    // `Send + Sync` (zero data races at compile time!).
    let shared_pool = Arc::new(pool);
    let mut handles = Vec::with_capacity(4);

    println!("\n[4] Simulating 4 concurrent worker threads routing 2 requests each:");
    for worker_id in 0..4 {
        // `Arc::clone` only increments an atomic reference counter (cheap, like copying a pointer),
        // it does NOT deep-copy the UpstreamPool.
        let pool_ref = Arc::clone(&shared_pool);

        let handle = thread::spawn(move || -> Result<Vec<String>, LbError> {
            let mut log = Vec::new();
            for req_num in 1..=2 {
                let chosen = pool_ref.pick_backend()?;
                log.push(format!(
                    "    [worker-{worker_id} | req #{req_num}] -> routed to {} (weight={})",
                    chosen.addr, chosen.weight
                ));
            }
            Ok(log)
        });
        handles.push(handle);
    }

    for handle in handles {
        let worker_logs = handle.join().expect("worker thread panicked")?;
        for entry in worker_logs {
            println!("{entry}");
        }
    }

    println!("\nNext step: Check out `labs/02-pingora-skeleton` to see how these concepts");
    println!("map directly onto Cloudflare Pingora's `Server`, `LoadBalancer`, and `ProxyHttp` traits!");

    Ok(())
}

/// Borrows a slice of backends (`&[Backend]`) read-only without taking ownership
/// or copying the underlying vector.
fn inspect_backends(backends: &[Backend]) {
    for b in backends {
        let state_desc = match &b.status {
            HealthStatus::Healthy => "HEALTHY (routable)".to_string(),
            HealthStatus::Degraded { reason } => format!("DEGRADED ({reason}) (routable)"),
            HealthStatus::Unhealthy {
                consecutive_failures,
            } => format!("UNHEALTHY ({consecutive_failures} consecutive failures) (skipped)"),
        };
        println!("    - {} [weight={}] -> {}", b.addr, b.weight, state_desc);
    }
}
