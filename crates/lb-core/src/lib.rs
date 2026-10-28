//! # `lb-core` — Load Balancer Core Primitives
//!
//! Educational library crate demonstrating Rust fundamentals for a Go developer
//! and Cloud Architect building toward a Cloudflare Pingora load balancer.
//!
//! ## Go vs. Rust Mental Model in this Crate
//! - **Package vs. Crate/Module**: In Go, a directory is a package. In Rust, a crate
//!   is a compilation unit (`lib.rs` or `main.rs`), and subdirectories/files are `mod`s.
//! - **Structs & Methods**: `struct` defines data; `impl` blocks attach methods and
//!   trait implementations (similar to Go receiver functions `func (b *Backend) ...`).
//! - **Enums (Sum Types)**: Rust `enum` variants can carry data (unlike Go's `iota`),
//!   and `match` forces exhaustive handling at compile time.
//! - **Traits vs. Interfaces**: Rust `trait`s are implemented explicitly (`impl Trait for Type`),
//!   whereas Go interfaces are satisfied implicitly (duck typing).
//! - **Error Handling**: Instead of `(val, err)` tuples, Rust returns `Result<T, E>`
//!   and propagates errors concisely with the `?` operator.

use std::fmt;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Represents the health state of an upstream backend node.
///
/// **Go comparison**: Unlike Go's `const` + `iota`, a Rust `enum` is an Algebraic
/// Data Type (sum type). The `Degraded` and `Unhealthy` variants carry contextual data
/// directly inside the enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { consecutive_failures: u32 },
}

impl HealthStatus {
    /// Returns `true` if the backend can accept traffic.
    pub fn is_routable(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded { .. })
    }
}

/// Domain errors for load balancer configuration and routing.
///
/// **Go comparison**: In Go you implement `Error() string`. In Rust you implement
/// `std::fmt::Display` and `std::error::Error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LbError {
    InvalidAddress(String),
    ZeroWeight { address: String },
    NonLoopbackBindDisallowed { address: SocketAddr },
    NoHealthyBackends,
}

impl fmt::Display for LbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddress(raw) => write!(f, "invalid socket address: '{raw}'"),
            Self::ZeroWeight { address } => {
                write!(f, "backend '{address}' must have weight >= 1")
            }
            Self::NonLoopbackBindDisallowed { address } => {
                write!(
                    f,
                    "listener address '{address}' rejected: local dev listeners must bind to loopback (127.0.0.1 / ::1)"
                )
            }
            Self::NoHealthyBackends => write!(f, "no healthy backends available in upstream pool"),
        }
    }
}

impl std::error::Error for LbError {}

/// Represents a single upstream backend server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backend {
    pub addr: SocketAddr,
    pub weight: u16,
    pub status: HealthStatus,
}

impl Backend {
    /// Creates and validates a new `Backend` from an `"ip:port"` string and weight.
    ///
    /// **Rust note**: There is no `nil` or default uninitialized constructor. By convention,
    /// `Type::new(...)` constructs a valid instance or returns `Result<Self, Error>`.
    pub fn new(addr_str: &str, weight: u16) -> Result<Self, LbError> {
        if weight == 0 {
            return Err(LbError::ZeroWeight {
                address: addr_str.to_string(),
            });
        }

        let addr: SocketAddr = addr_str
            .parse()
            .map_err(|_| LbError::InvalidAddress(addr_str.to_string()))?;

        Ok(Self {
            addr,
            weight,
            status: HealthStatus::Healthy,
        })
    }

    /// Builder-style helper to set initial health status.
    ///
    /// **Ownership note**: `mut self` takes ownership of `self` by value, mutates it,
    /// and returns the owned value back — enabling ergonomic method chaining.
    pub fn with_status(mut self, status: HealthStatus) -> Self {
        self.status = status;
        self
    }
}

/// Validates that a local test/dev proxy listener binds strictly to localhost (`127.0.0.1` or `::1`).
///
/// Following secure-by-default development practices, local test servers must not bind to `0.0.0.0`.
pub fn validate_local_listener_addr(addr_str: &str) -> Result<SocketAddr, LbError> {
    let addr: SocketAddr = addr_str
        .parse()
        .map_err(|_| LbError::InvalidAddress(addr_str.to_string()))?;

    if !addr.ip().is_loopback() {
        return Err(LbError::NonLoopbackBindDisallowed { address: addr });
    }
    Ok(addr)
}

/// Strategy trait for choosing an upstream backend from a slice of candidates.
///
/// **Go comparison**: Equivalent to `type SelectionStrategy interface { Select(backends []Backend) (*Backend, error) }`,
/// except Rust borrows a slice `&[Backend]` and returns a borrowed reference `&'a Backend`
/// tied to the lifetime `'a` of the input slice — zero heap allocations!
pub trait SelectionStrategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn select<'a>(&self, backends: &'a [Backend]) -> Result<&'a Backend, LbError>;
}

/// Lock-free Round-Robin backend selector using an atomic counter.
///
/// **Go comparison**: Like using `atomic.Uint64` inside a struct. Notice that `select`
/// takes `&self` (shared immutable reference), yet `AtomicUsize::fetch_add` safely mutates
/// the counter across threads via *interior mutability*.
#[derive(Debug, Default)]
pub struct RoundRobinSelector {
    counter: AtomicUsize,
}

impl RoundRobinSelector {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl SelectionStrategy for RoundRobinSelector {
    fn name(&self) -> &'static str {
        "round-robin"
    }

    fn select<'a>(&self, backends: &'a [Backend]) -> Result<&'a Backend, LbError> {
        // Count how many routable backends exist without allocating a new Vec.
        let routable_count = backends.iter().filter(|b| b.status.is_routable()).count();
        if routable_count == 0 {
            return Err(LbError::NoHealthyBackends);
        }

        let ticket = self.counter.fetch_add(1, Ordering::Relaxed);
        let target_idx = ticket % routable_count;

        backends
            .iter()
            .filter(|b| b.status.is_routable())
            .nth(target_idx)
            .ok_or(LbError::NoHealthyBackends)
    }
}

/// An upstream pool combining a list of backends with a pluggable selection strategy.
///
/// Uses a generic type parameter `S: SelectionStrategy` for zero-cost static dispatch
/// (monomorphization at compile time, rather than vtable interface boxing).
pub struct UpstreamPool<S: SelectionStrategy> {
    name: String,
    backends: Vec<Backend>,
    strategy: S,
}

impl<S: SelectionStrategy> UpstreamPool<S> {
    pub fn new(name: impl Into<String>, backends: Vec<Backend>, strategy: S) -> Self {
        Self {
            name: name.into(),
            backends,
            strategy,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn strategy_name(&self) -> &'static str {
        self.strategy.name()
    }

    pub fn backends(&self) -> &[Backend] {
        &self.backends
    }

    /// Selects the next routable backend according to the pool's strategy.
    pub fn pick_backend(&self) -> Result<&Backend, LbError> {
        self.strategy.select(&self.backends)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_validation_rejects_zero_weight_and_bad_addr() {
        assert_eq!(
            Backend::new("127.0.0.1:8081", 0),
            Err(LbError::ZeroWeight {
                address: "127.0.0.1:8081".to_string()
            })
        );
        assert!(matches!(
            Backend::new("not-an-ip", 1),
            Err(LbError::InvalidAddress(_))
        ));
    }

    #[test]
    fn local_listener_rejects_wildcard_bind() {
        assert!(validate_local_listener_addr("127.0.0.1:6188").is_ok());
        assert!(matches!(
            validate_local_listener_addr("0.0.0.0:6188"),
            Err(LbError::NonLoopbackBindDisallowed { .. })
        ));
    }

    #[test]
    fn round_robin_skips_unhealthy_nodes() {
        let backends = vec![
            Backend::new("127.0.0.1:8081", 1).unwrap(),
            Backend::new("127.0.0.1:8082", 1)
                .unwrap()
                .with_status(HealthStatus::Unhealthy {
                    consecutive_failures: 3,
                }),
            Backend::new("127.0.0.1:8083", 1)
                .unwrap()
                .with_status(HealthStatus::Degraded {
                    reason: "high latency".to_string(),
                }),
        ];

        let pool = UpstreamPool::new("api-upstream", backends, RoundRobinSelector::new());

        let first = pool.pick_backend().unwrap();
        let second = pool.pick_backend().unwrap();
        let third = pool.pick_backend().unwrap();

        assert_eq!(first.addr.port(), 8081);
        assert_eq!(second.addr.port(), 8083);
        assert_eq!(third.addr.port(), 8081);
    }
}
