//! This crate provides a HTTP server that is solely dedicated to serving the `/metrics` endpoint.
//!
//! For other endpoints, see the `http_api` crate.
pub mod metrics;

use crate::validation::{DutiesService, ValidatorStore};
use lighthouse_version::version_with_platform;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use slog::{crit, info, Logger};
use slot_clock::SystemTimeSlotClock;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use types::EthSpec;
use warp::{http::Response, Filter};

#[derive(Debug)]
pub enum Error {
    Warp(warp::Error),
    Other(String),
}

impl From<warp::Error> for Error {
    fn from(e: warp::Error) -> Self {
        Error::Warp(e)
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::Other(e)
    }
}

/// Contains objects which have shared access from inside/outside of the metrics server.
pub struct Shared<T: EthSpec> {
    pub validator_store: Option<Arc<ValidatorStore<SystemTimeSlotClock, T>>>,
    pub duties_service: Option<Arc<DutiesService<SystemTimeSlotClock, T>>>,
    pub genesis_time: Option<u64>,
}

/// A wrapper around all the items required to spawn the HTTP server.
///
/// The server will gracefully handle the case where any fields are `None`.
pub struct Context<T: EthSpec> {
    pub config: Config,
    pub shared: RwLock<Shared<T>>,
    pub log: Logger,
}

/// Configuration for the HTTP server.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub listen_addr: IpAddr,
    pub listen_port: u16,
    pub allow_origin: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            listen_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            listen_port: 5064,
            allow_origin: None,
        }
    }
}

/// Creates a server that will serve requests using information from `ctx`.
///
/// The server will shut down gracefully when the `shutdown` future resolves.
///
/// ## Returns
///
/// This function will bind the server to the provided address and then return a tuple of:
///
/// - `SocketAddr`: the address that the HTTP server will listen on.
/// - `Future`: the actual server future that will need to be awaited.
///
/// ## Errors
///
/// Returns an error if the server is unable to bind or there is another error during
/// configuration.
pub fn serve<T: EthSpec>(
    ctx: Arc<Context<T>>,
    shutdown: impl Future<Output = ()> + Send + Sync + 'static,
) -> Result<(SocketAddr, impl Future<Output = ()>), Error> {
    let config = &ctx.config;
    let log = ctx.log.clone();

    // Configure CORS.
    let cors_builder = {
        let builder = warp::cors()
            .allow_method("GET")
            .allow_headers(vec!["Content-Type"]);

        warp_utils::cors::set_builder_origins(
            builder,
            config.allow_origin.as_deref(),
            (config.listen_addr, config.listen_port),
        )?
    };

    // Sanity check.
    if !config.enabled {
        crit!(log, "Cannot start disabled metrics HTTP server");
        return Err(Error::Other(
            "A disabled metrics server should not be started".to_string(),
        ));
    }

    let inner_ctx = ctx.clone();
    let routes = warp::get()
        .and(warp::path("metrics"))
        .map(move || inner_ctx.clone())
        .and_then(|ctx: Arc<Context<T>>| async move {
            Ok::<_, warp::Rejection>(
                metrics::gather_prometheus_metrics(&ctx).await
                    .map(|body| Response::builder().status(200).body(body).unwrap())
                    .unwrap_or_else(|e| {
                        Response::builder()
                            .status(500)
                            .header("Content-Type", "text/plain")
                            .body(format!("Unable to gather metrics: {:?}", e))
                            .unwrap()
                    }),
            )
        })
        // Add a `Server` header.
        .map(|reply| warp::reply::with_header(reply, "Server", &version_with_platform()))
        .with(cors_builder.build());

    let (listening_socket, server) = warp::serve(routes).try_bind_with_graceful_shutdown(
        SocketAddr::new(config.listen_addr, config.listen_port),
        async {
            shutdown.await;
        },
    )?;

    info!(
        log,
        "Metrics HTTP server started";
        "listen_address" => listening_socket.to_string(),
    );

    Ok((listening_socket, server))
}

