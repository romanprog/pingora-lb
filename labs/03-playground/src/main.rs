use std::time::Duration;
use tokio::time::{sleep, timeout};

// Simulated network connection struct
struct FakeChannel {
    endpoint: String,
}

// Simulates an unstable server:
// Attempt 1 & 2: hangs for 2 seconds (triggers our 300ms timeout!)
// Attempt 3: fails immediately with "Connection refused"
// Attempt 4+: succeeds in 50ms!
async fn fake_grpc_connect(endpoint: &str, attempt: u32) -> Result<FakeChannel, String> {
    match attempt {
        1 | 2 => {
            // Simulate a blackhole firewall (packet drop / hang)
            sleep(Duration::from_secs(2)).await;
            Ok(FakeChannel {
                endpoint: endpoint.to_string(),
            })
        }
        3 => {
            // Simulate fast TCP RST (Connection refused)
            sleep(Duration::from_millis(20)).await;
            Err("OS error 111: Connection refused".to_string())
        }
        _ => {
            // Healthy fast connection
            sleep(Duration::from_millis(50)).await;
            Ok(FakeChannel {
                endpoint: endpoint.to_string(),
            })
        }
    }
}

// =========================================================================
// RETRY HELPER WITH TIMEOUT AND EXPONENTIAL BACKOFF
// =========================================================================
async fn connect_with_retry(
    endpoint: &str,
    conn_timeout: Duration,
    max_attempts: u32,
) -> Result<FakeChannel, String> {
    let mut backoff = Duration::from_millis(100);
    let max_backoff = Duration::from_secs(2);

    for attempt in 1..=max_attempts {
        println!("[Attempt {attempt}/{max_attempts}] Connecting to {endpoint}...");

        // Wrap the async connect call in `tokio::time::timeout`
        match timeout(conn_timeout, fake_grpc_connect(endpoint, attempt)).await {
            // Case 1: Completed within timeout, AND connection succeeded!
            Ok(Ok(channel)) => {
                println!("[Attempt {attempt}] Connected successfully!");
                return Ok(channel);
            }

            // Case 2: Completed within timeout, BUT server returned an error
            Ok(Err(net_err)) => {
                println!("[Attempt {attempt}] Network error: {net_err}");
            }

            // Case 3: Timer expired! `fake_grpc_connect` future was automatically dropped & cancelled.
            Err(_elapsed) => {
                println!(
                    "[Attempt {attempt}] Timed out after {:?}! Future cancelled.",
                    conn_timeout
                );
            }
        }

        if attempt == max_attempts {
            break;
        }

        println!("   -> Sleeping {:?} before next retry...\n", backoff);
        sleep(backoff).await;

        // Exponential backoff: double the wait time up to `max_backoff`
        backoff = (backoff * 2).min(max_backoff);
    }

    Err(format!("Failed to connect after {max_attempts} attempts"))
}

#[tokio::main]
async fn main() {
    let conn_timeout = Duration::from_millis(300);

    match connect_with_retry("http://10.0.0.1:50051", conn_timeout, 5).await {
        Ok(channel) => {
            println!("\nReady to start gRPC sync stream on {}", channel.endpoint);
        }
        Err(fatal) => {
            println!("\nFatal error: {fatal}");
        }
    }
}