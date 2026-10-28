//! # Lab 02: Cloudflare Pingora Load Balancer Blueprint
//!
//! This sub-project bridges the core Rust concepts from `lb-core` / `lab-01-rust-basics`
//! directly to Cloudflare's `pingora` framework architecture.
//!
//! ## How Pingora Works (for a Cloud Architect & Go Developer)
//! In Go (`net/http/httputil.ReverseProxy`), you configure a `Director` function and `Transport`.
//! In Pingora, you define a struct holding your `Arc<LoadBalancer<RoundRobin>>` and implement
//! the `ProxyHttp` async trait, which gives you fine-grained hooks into every stage of the HTTP
//! request lifecycle:
//!
//! 1. `new_ctx(&self) -> Self::CTX` — Per-request context (like Go's `context.Context` state, but strongly typed!).
//! 2. `request_filter(&self, session, ctx) -> Result<bool>` — Early WAF/auth/rate-limit check before upstream selection.
//! 3. `upstream_peer(&self, session, ctx) -> Result<Box<HttpPeer>>` — Selects the backend from `LoadBalancer` and configures TLS/SNI/Timeouts.
//! 4. `upstream_request_filter(&self, session, upstream_req, ctx)` — Mutates headers sent to upstream (e.g., `Host`, `X-Forwarded-For`).
//! 5. `logging(&self, session, err, ctx)` — Emits structured metrics/logs after the request finishes.

use std::sync::Arc;
use std::time::Instant;

use lb_core::{
    validate_local_listener_addr, Backend, LbError, RoundRobinSelector, UpstreamPool,
};

/// Per-request strongly-typed context (equivalent to `ProxyHttp::CTX` in Pingora).
pub struct RequestCtx {
    pub request_id: u64,
    pub start_time: Instant,
    pub selected_backend: Option<String>,
}

/// Blueprint of a Pingora `ProxyHttp` service.
pub struct PingoraLbBlueprint {
    pub upstreams: Arc<UpstreamPool<RoundRobinSelector>>,
}

impl PingoraLbBlueprint {
    pub fn new(upstreams: Arc<UpstreamPool<RoundRobinSelector>>) -> Self {
        Self { upstreams }
    }

    /// Mirrors Pingora's `ProxyHttp::new_ctx(&self)`
    pub fn new_ctx(&self, request_id: u64) -> RequestCtx {
        RequestCtx {
            request_id,
            start_time: Instant::now(),
            selected_backend: None,
        }
    }

    /// Mirrors Pingora's `ProxyHttp::upstream_peer(&self, session, ctx)`
    pub fn upstream_peer(&self, ctx: &mut RequestCtx) -> Result<String, LbError> {
        let backend = self.upstreams.pick_backend()?;
        let peer_addr = backend.addr.to_string();
        ctx.selected_backend = Some(peer_addr.clone());
        Ok(peer_addr)
    }
}

fn main() -> Result<(), LbError> {
    println!("==================================================================");
    println!("  Lab 02: Cloudflare Pingora ProxyHttp Lifecycle Blueprint        ");
    println!("==================================================================\n");

    let listen_addr = validate_local_listener_addr("127.0.0.1:6188")?;

    let backends = vec![
        Backend::new("127.0.0.1:10080", 1)?,
        Backend::new("127.0.0.1:10081", 1)?,
    ];
    let pool = Arc::new(UpstreamPool::new(
        "origin-cluster",
        backends,
        RoundRobinSelector::new(),
    ));

    let lb_service = PingoraLbBlueprint::new(pool);
    println!("Proxy listener configured on http://{listen_addr}");
    println!("Simulating Pingora `ProxyHttp` request pipeline:\n");

    for req_id in 1..=3 {
        let mut ctx = lb_service.new_ctx(req_id);
        let peer = lb_service.upstream_peer(&mut ctx)?;
        let elapsed = ctx.start_time.elapsed();
        println!(
            "  [req #{req_id}] new_ctx() -> upstream_peer() selected {peer} -> completed in {elapsed:?}"
        );
    }

    println!("\n------------------------------------------------------------------");
    println!("Ready to enable real `pingora` crate?");
    println!("1. Install C/C++ build tools required by Pingora's TLS layer:");
    println!("   sudo apt update && sudo apt install -y cmake clang libclang-dev pkg-config libssl-dev");
    println!("2. Uncomment `pingora` and `async-trait` in Cargo.toml and see the reference");
    println!("   implementation at the bottom of `labs/02-pingora-skeleton/src/main.rs`!");
    println!("------------------------------------------------------------------");

    Ok(())
}

// ============================================================================
// REFERENCE: Real Cloudflare Pingora 0.4 Implementation
// (Uncomment once `pingora` and `async-trait` are enabled in Cargo.toml)
// ============================================================================
/*
use async_trait::async_trait;
use pingora::lb::{selection::RoundRobin, LoadBalancer};
use pingora::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub struct MyPingoraLb(pub Arc<LoadBalancer<RoundRobin>>);

#[async_trait]
impl ProxyHttp for MyPingoraLb {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
        let upstream = self
            .0
            .select(b"", 256) // hash key doesn't matter for RoundRobin
            .ok_or_else(|| Error::new_str("No healthy upstream available"))?;

        // Connect to upstream (false = plaintext HTTP, true = HTTPS with SNI)
        let peer = Box::new(HttpPeer::new(upstream, false, "one.one.one.one".to_string()));
        Ok(peer)
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request.insert_header("Host", "one.one.one.one")?;
        Ok(())
    }
}

pub fn run_pingora_server() {
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    let mut upstreams = LoadBalancer::try_from_iter(["1.1.1.1:80", "1.0.0.1:80"]).unwrap();

    // Attach TCP health check running every 1 second in the background
    let hc = TcpHealthCheck::new();
    upstreams.set_health_check(hc);
    upstreams.health_check_frequency = Some(Duration::from_secs(1));

    let background = background_service("health check", upstreams);
    let upstreams = background.task();

    let mut lb = http_proxy_service(&my_server.configuration, MyPingoraLb(upstreams));
    // Bind strictly to localhost during local development/testing
    lb.add_tcp("127.0.0.1:6188");

    my_server.add_service(background);
    my_server.add_service(lb);
    my_server.run_forever();
}
*/
