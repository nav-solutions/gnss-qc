//! NTRIP clients, as real-time inputs of the MISO engine.
//!
//! A [QcNtripSource] connects to one mount point of an NTRIP caster, with its own
//! connection settings ([QcNtripConfig]), and streams the RTCM 3 messages it receives into the
//! engine, decoded by the [QcRtcmDecoder]: signal observations (code, phase, doppler, C/N0),
//! radio ephemerides and the coordinates of the reference station. A stream therefore plays the
//! role of an observation RINEX and of a navigation RINEX at the same time, and contributes to
//! every processor of the engine (signals, ephemerides, navigation..).
//!
//! Deploy as many clients as needed (rover, RTK base, ephemerides streams..): each client is an
//! independent [QcSource] of the engine, which synchronizes them epoch by epoch, like files.
//!
//! ```text
//!  NTRIP rover  (mount A) ──┐
//!  NTRIP base   (mount B) ──┼──► QcEngine ──► processors (signals, navigation, RTK..)
//!  NTRIP BCEP   (mount C) ──┤
//!  SP3 / CLK files       ───┘
//! ```
//!
//! ## Indexing
//! A mount point streams the data of one station, so the stream is indexed like a file:
//! by default, by its mount point name ([QcIndexing::Custom]), or by any [QcIndexing]
//! ([QcNtripConfig::with_indexing]). The identity of a signal source must be unique.
//!
//! ## Roles
//! The [QcNtripRole] declares what the stream contributes to. It tells the engine whether this
//! source delivers signals (rover, base) and must be waited for, epoch after epoch,
//! or only provides context (ephemerides stream).
//!
//! ## Runtime
//! Each client runs in its own background thread, with its own (single threaded) Tokio runtime:
//! the engine itself remains runtime agnostic and is never blocked by a client
//! ([QcPoll::Pending] while waiting for data). The client stops when the [QcNtripSource]
//! is dropped, or when the caster closes the connection (end of stream).
//!
//! ## Reconnection
//! By default, a connection failure is reported to the engine (a [QcSourceError]) and a
//! closed connection ends the stream. Long running real-time services should rather reconnect
//! ([QcNtripConfig::with_reconnection]): the client then retries forever, the delay doubling
//! after each failure (up to 2 minutes), and the stream continues from where the caster
//! resumes. Combine it with [crate::prelude::QcEngineBuilder::with_max_latency], so that
//! a station that is reconnecting does not hold the other ones.
//!
//! ## Monitoring
//! [QcNtripSource::monitor] returns a handle on the state of the client (connection, decoded
//! messages), that remains available once the source is moved into the engine.
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

use futures::{channel::oneshot, future::Either, StreamExt};
use hifitime::Epoch;
use ntrip_client::{NtripClient, NtripClientError, NtripConfig, NtripCredentials};
use thiserror::Error;

#[cfg(feature = "log")]
use log::{debug, error, info, warn};

use crate::io::{
    input::indexing::QcIndexing,
    rtcm::{QcRtcmDecoder, QcRtcmStats},
    source::{QcInputItem, QcPoll, QcSource, QcSourceDescriptor, QcSourceError},
};

/// Default NTRIP port
pub const DEFAULT_NTRIP_PORT: u16 = 2101;

/// NTRIP client errors
#[derive(Debug, Clone, Error, PartialEq)]
pub enum QcNtripError {
    #[error("invalid NTRIP URL \"{0}\": expecting ntrip://[user[:password]@]host[:port]/MOUNT")]
    InvalidUrl(String),

    #[error("invalid NTRIP port \"{0}\"")]
    InvalidPort(String),

    #[error("missing NTRIP mount point")]
    MissingMountPoint,

    #[error("invalid NTRIP role \"{0}\": expecting rover, base or ephemerides")]
    InvalidRole(String),

    #[error("NTRIP connection failure: {reason}")]
    Connection { mount: String, reason: String },

    #[error("failed to deploy the NTRIP client: {0}")]
    Runtime(String),
}

/// What an NTRIP stream contributes to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcNtripRole {
    /// Receiver to process or navigate: signal observations
    /// (and possibly radio ephemerides)
    #[default]
    Rover,

    /// Reference station (RTK base): signal observations, coordinates of the station
    /// (RTCM 1005/1006) and possibly radio ephemerides
    Base,

    /// Radio ephemerides only (broadcast ephemerides streams, like `BCEP00BKG0`).
    /// The engine never waits for this stream.
    Ephemerides,
}

impl QcNtripRole {
    /// Returns true if this stream delivers signal observations
    pub fn provides_signals(&self) -> bool {
        matches!(self, Self::Rover | Self::Base)
    }
}

impl std::fmt::Display for QcNtripRole {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Rover => write!(f, "rover"),
            Self::Base => write!(f, "base"),
            Self::Ephemerides => write!(f, "ephemerides"),
        }
    }
}

impl FromStr for QcNtripRole {
    type Err = QcNtripError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "rover" => Ok(Self::Rover),
            "base" | "rtk" | "reference" => Ok(Self::Base),
            "ephemerides" | "ephemeris" | "nav" | "navigation" => Ok(Self::Ephemerides),
            _ => Err(QcNtripError::InvalidRole(s.to_string())),
        }
    }
}

/// Connection settings of one NTRIP client.
///
/// ```
/// use gnss_qc::prelude::{QcIndexing, QcNtripConfig, QcNtripRole};
///
/// let cfg = "ntrip://user:secret@caster.centipede.fr:2101/MOJN"
///     .parse::<QcNtripConfig>()
///     .unwrap()
///     .with_role(QcNtripRole::Base);
///
/// assert_eq!(cfg.host, "caster.centipede.fr");
/// assert_eq!(cfg.port, 2101);
/// assert_eq!(cfg.mount, "MOJN");
/// assert_eq!(cfg.user.as_deref(), Some("user"));
/// assert_eq!(cfg.indexing(), QcIndexing::Custom("MOJN".to_string()));
/// ```
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcNtripConfig {
    /// Host name or IP address of the caster
    pub host: String,

    /// Port of the caster
    #[cfg_attr(feature = "serde", serde(default = "default_port"))]
    pub port: u16,

    /// TLS connection
    #[cfg_attr(feature = "serde", serde(default))]
    pub use_tls: bool,

    /// Mount point (stream name, usually the name of the station)
    pub mount: String,

    /// User name, when the caster requires credentials
    #[cfg_attr(feature = "serde", serde(default))]
    pub user: Option<String>,

    /// Password, when the caster requires credentials
    #[cfg_attr(feature = "serde", serde(default))]
    pub password: Option<String>,

    /// Identity of this stream in the engine. Default: mount point ([QcIndexing::Custom]).
    #[cfg_attr(feature = "serde", serde(default))]
    pub indexing: Option<QcIndexing>,

    /// What this stream contributes to
    #[cfg_attr(feature = "serde", serde(default))]
    pub role: QcNtripRole,

    /// Reference position of the antenna (ARP, ECEF m), when known in advance.
    /// Otherwise it is learned from the stream (RTCM 1005/1006).
    #[cfg_attr(feature = "serde", serde(default))]
    pub reference_position_ecef_m: Option<(f64, f64, f64)>,

    /// Approximate date of the stream, to resolve the GNSS weeks of the time tags.
    /// Default: system time (live streams). Define it to replay recorded streams.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reference_epoch: Option<Epoch>,

    /// Reconnects after a connection failure, or when the caster closes the connection,
    /// after this initial delay (doubled after each failure, up to 2 minutes).
    /// Default: no reconnection (failures are reported, the stream ends).
    #[cfg_attr(feature = "serde", serde(default))]
    pub reconnection: Option<std::time::Duration>,
}

#[cfg(feature = "serde")]
fn default_port() -> u16 {
    DEFAULT_NTRIP_PORT
}

impl std::fmt::Debug for QcNtripConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QcNtripConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("use_tls", &self.use_tls)
            .field("mount", &self.mount)
            .field("user", &self.user)
            .field("password", &self.password.as_ref().map(|_| "********"))
            .field("indexing", &self.indexing)
            .field("role", &self.role)
            .field("reference_position_ecef_m", &self.reference_position_ecef_m)
            .field("reference_epoch", &self.reference_epoch)
            .field("reconnection", &self.reconnection)
            .finish()
    }
}

impl std::fmt::Display for QcNtripConfig {
    /// `ntrip://[user@]host:port/MOUNT` (the password is never displayed)
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let scheme = if self.use_tls { "ntrips" } else { "ntrip" };
        match &self.user {
            Some(user) => write!(
                f,
                "{}://{}@{}:{}/{}",
                scheme, user, self.host, self.port, self.mount
            ),
            None => write!(f, "{}://{}:{}/{}", scheme, self.host, self.port, self.mount),
        }
    }
}

impl QcNtripConfig {
    /// New [QcNtripConfig] (no credentials, no TLS), for this caster and mount point.
    pub fn new(host: &str, port: u16, mount: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            use_tls: false,
            mount: mount.to_string(),
            user: None,
            password: None,
            indexing: None,
            role: QcNtripRole::default(),
            reference_position_ecef_m: None,
            reference_epoch: None,
            reconnection: None,
        }
    }

    /// Identity of this stream in the engine
    pub fn indexing(&self) -> QcIndexing {
        self.indexing
            .clone()
            .unwrap_or_else(|| QcIndexing::Custom(self.mount.clone()))
    }

    /// Copies and returns with credentials
    pub fn with_credentials(mut self, user: &str, password: &str) -> Self {
        self.user = Some(user.to_string());
        self.password = Some(password.to_string());
        self
    }

    /// Copies and returns with TLS connection
    pub fn with_tls(mut self) -> Self {
        self.use_tls = true;
        self
    }

    /// Copies and returns with this identity in the engine
    pub fn with_indexing(mut self, indexing: QcIndexing) -> Self {
        self.indexing = Some(indexing);
        self
    }

    /// Copies and returns with this [QcNtripRole]
    pub fn with_role(mut self, role: QcNtripRole) -> Self {
        self.role = role;
        self
    }

    /// Copies and returns with this reference position (ARP, ECEF m)
    pub fn with_reference_position_ecef_m(mut self, position: (f64, f64, f64)) -> Self {
        self.reference_position_ecef_m = Some(position);
        self
    }

    /// Copies and returns with this approximate date (recorded streams)
    pub fn with_reference_epoch(mut self, epoch: Epoch) -> Self {
        self.reference_epoch = Some(epoch);
        self
    }

    /// Copies and returns with automatic reconnection: after a connection failure, or when
    /// the caster closes the connection, the client reconnects after `delay`
    /// (doubled after each failure, up to 2 minutes).
    pub fn with_reconnection(mut self, delay: std::time::Duration) -> Self {
        self.reconnection = Some(delay);
        self
    }

    /// [QcSourceDescriptor] of this stream
    pub fn descriptor(&self) -> QcSourceDescriptor {
        let mut descriptor = QcSourceDescriptor::new(self.indexing()).with_ephemerides();

        if self.role.provides_signals() {
            descriptor = descriptor.with_signals();
        }

        if let Some(position) = self.reference_position_ecef_m {
            descriptor = descriptor.with_reference_position_ecef_m(position);
        }

        descriptor
    }
}

impl FromStr for QcNtripConfig {
    type Err = QcNtripError;

    /// Parses `ntrip://[user[:password]@]host[:port]/MOUNT`.
    /// `ntrips://` (or `https://`) selects a TLS connection, `http://` is also accepted,
    /// the scheme may be omitted. Default port: 2101.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || QcNtripError::InvalidUrl(s.to_string());

        let (use_tls, rest) = match s.trim().split_once("://") {
            Some((scheme, rest)) => match scheme.to_lowercase().as_str() {
                "ntrip" | "http" => (false, rest),
                "ntrips" | "https" => (true, rest),
                _ => return Err(invalid()),
            },
            None => (false, s.trim()),
        };

        let (authority, mount) = rest
            .split_once('/')
            .ok_or(QcNtripError::MissingMountPoint)?;

        let mount = mount.trim_end_matches('/');
        if mount.is_empty() {
            return Err(QcNtripError::MissingMountPoint);
        }

        let (credentials, address) = match authority.rsplit_once('@') {
            Some((credentials, address)) => (Some(credentials), address),
            None => (None, authority),
        };

        let (host, port) = match address.rsplit_once(':') {
            Some((host, port)) => (
                host,
                port.parse::<u16>()
                    .map_err(|_| QcNtripError::InvalidPort(port.to_string()))?,
            ),
            None => (address, DEFAULT_NTRIP_PORT),
        };

        if host.is_empty() {
            return Err(invalid());
        }

        let mut cfg = Self::new(host, port, mount);
        cfg.use_tls = use_tls;

        if let Some(credentials) = credentials {
            let (user, password) = match credentials.split_once(':') {
                Some((user, password)) => (user, password),
                None => (credentials, ""),
            };
            cfg.user = Some(user.to_string());
            cfg.password = Some(password.to_string());
        }

        Ok(cfg)
    }
}

/// Messages from the client thread
type ClientMessage = Result<QcInputItem, QcNtripError>;

/// Longest delay between two reconnection attempts
const MAX_RECONNECTION_DELAY: std::time::Duration = std::time::Duration::from_secs(120);

/// State of an NTRIP client, available once its [QcNtripSource] was moved into the engine
/// ([QcNtripSource::monitor]).
#[derive(Debug, Clone)]
pub struct QcNtripMonitor {
    cfg: QcNtripConfig,
    stats: Arc<Mutex<QcRtcmStats>>,
    connected: Arc<AtomicBool>,
    connections: Arc<std::sync::atomic::AtomicUsize>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl QcNtripMonitor {
    /// Connection settings of the client
    pub fn config(&self) -> &QcNtripConfig {
        &self.cfg
    }

    /// Returns true while the caster streams
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Number of successful connections (reconnections included)
    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::Relaxed)
    }

    /// Latest connection failure, if any
    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|e| e.clone())
    }

    /// Statistics of the decoded stream (so far)
    pub fn stats(&self) -> QcRtcmStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// Real-time [QcSource]: one NTRIP client, see the [module](crate::io::ntrip) documentation.
pub struct QcNtripSource {
    cfg: QcNtripConfig,
    descriptor: QcSourceDescriptor,
    receiver: Receiver<ClientMessage>,
    monitor: QcNtripMonitor,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl QcNtripSource {
    /// Deploys a new NTRIP client in a background thread. The connection is established
    /// asynchronously: connection failures are reported by the first [QcSource::poll]
    /// that follows.
    pub fn new(cfg: QcNtripConfig) -> Result<Self, QcNtripError> {
        let descriptor = cfg.descriptor();

        let (sender, receiver) = channel();
        let (shutdown, shutdown_rx) = oneshot::channel();

        let monitor = QcNtripMonitor {
            cfg: cfg.clone(),
            stats: Arc::new(Mutex::new(QcRtcmStats::default())),
            connected: Arc::new(AtomicBool::new(false)),
            connections: Default::default(),
            last_error: Default::default(),
        };

        let client = Client {
            cfg: cfg.clone(),
            sender,
            monitor: monitor.clone(),
        };

        let thread = std::thread::Builder::new()
            .name(format!("ntrip-{}", cfg.mount))
            .spawn(move || client.run(shutdown_rx))
            .map_err(|e| QcNtripError::Runtime(e.to_string()))?;

        Ok(Self {
            cfg,
            descriptor,
            receiver,
            monitor,
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
    }

    /// Connection settings of this client
    pub fn config(&self) -> &QcNtripConfig {
        &self.cfg
    }

    /// Returns true while the caster streams
    pub fn is_connected(&self) -> bool {
        self.monitor.is_connected()
    }

    /// Statistics of the decoded stream (so far)
    pub fn stats(&self) -> QcRtcmStats {
        self.monitor.stats()
    }

    /// Handle on the state of this client, that remains available once the source
    /// is moved into the engine
    pub fn monitor(&self) -> QcNtripMonitor {
        self.monitor.clone()
    }
}

impl QcSource for QcNtripSource {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        match self.receiver.try_recv() {
            Ok(Ok(item)) => Ok(QcPoll::Ready(item)),
            Ok(Err(error)) => Err(Box::new(error)),
            Err(TryRecvError::Empty) => Ok(QcPoll::Pending),
            Err(TryRecvError::Disconnected) => Ok(QcPoll::EndOfStream),
        }
    }
}

impl Drop for QcNtripSource {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        // the client thread exits on its own (shutdown signal, or blocked on a caster that
        // does not respond): it is never joined, not to block the application.
        drop(self.thread.take());
    }
}

/// The client, running in its own thread
struct Client {
    cfg: QcNtripConfig,
    sender: std::sync::mpsc::Sender<ClientMessage>,
    monitor: QcNtripMonitor,
}

/// End of one connection
enum Session {
    /// The connection failed, or the caster closed it
    Failed(QcNtripError),
    /// The caster closed the connection (end of stream)
    Ended,
    /// Shutdown requested, or the source was dropped
    Stopped,
}

impl Client {
    fn run(self, shutdown: oneshot::Receiver<()>) {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => {
                let _ = self.sender.send(Err(QcNtripError::Runtime(e.to_string())));
                return;
            }
        };

        runtime.block_on(self.stream(shutdown));
    }

    async fn stream(self, mut shutdown: oneshot::Receiver<()>) {
        let mut decoder = match self.cfg.reference_epoch {
            Some(epoch) => QcRtcmDecoder::new().with_reference_epoch(epoch),
            None => QcRtcmDecoder::new(),
        };

        let mut delay = self.cfg.reconnection;

        loop {
            let connections = self.monitor.connections();
            let session = self.session(&mut decoder, &mut shutdown).await;
            let streamed = self.monitor.connections() > connections;

            self.monitor.connected.store(false, Ordering::Relaxed);

            let error = match session {
                Session::Stopped => return,
                Session::Ended => None,
                Session::Failed(error) => {
                    if let Ok(mut last) = self.monitor.last_error.lock() {
                        *last = Some(error.to_string());
                    }
                    Some(error)
                }
            };

            let Some(wait) = delay else {
                // no reconnection: failures are reported, the stream ends
                if let Some(error) = error {
                    let _ = self.sender.send(Err(error));
                }
                return;
            };

            #[cfg(feature = "log")]
            warn!(
                "{}: {}, reconnecting in {:?}",
                self.cfg,
                error
                    .as_ref()
                    .map(|e| e.to_string())
                    .unwrap_or("end of stream".to_string()),
                wait
            );

            let sleep = Box::pin(tokio::time::sleep(wait));
            if let Either::Right(_) = futures::future::select(sleep, &mut shutdown).await {
                return;
            }

            // the next failure waits longer, unless the caster accepted the connection
            delay = Some(if streamed {
                self.cfg.reconnection.unwrap_or(wait)
            } else {
                (wait * 2).min(MAX_RECONNECTION_DELAY)
            });
        }
    }

    /// One connection, until it fails or ends
    async fn session(
        &self,
        decoder: &mut QcRtcmDecoder,
        shutdown: &mut oneshot::Receiver<()>,
    ) -> Session {
        let mount = self.cfg.mount.clone();

        let failure = |error: NtripClientError| QcNtripError::Connection {
            mount: mount.clone(),
            reason: match error {
                NtripClientError::ResponseError(status) => {
                    format!("caster response \"{}\"", status)
                }
                other => other.to_string(),
            },
        };

        let config = NtripConfig {
            host: self.cfg.host.clone(),
            port: self.cfg.port,
            use_tls: self.cfg.use_tls,
        };

        let credentials = NtripCredentials {
            user: self.cfg.user.clone().unwrap_or_default(),
            pass: self.cfg.password.clone().unwrap_or_default(),
        };

        #[cfg(feature = "log")]
        debug!("{}: connecting", self.cfg);

        let mut handle = match NtripClient::new(config, credentials).await {
            Ok(mut client) => match client.mount(&mount).await {
                Ok(handle) => handle,
                Err(e) => {
                    #[cfg(feature = "log")]
                    error!("{}: {}", self.cfg, e);
                    return Session::Failed(failure(e));
                }
            },
            Err(e) => {
                #[cfg(feature = "log")]
                error!("{}: {}", self.cfg, e);
                return Session::Failed(failure(e));
            }
        };

        self.monitor.connected.store(true, Ordering::Relaxed);
        self.monitor.connections.fetch_add(1, Ordering::Relaxed);

        #[cfg(feature = "log")]
        info!("{}: connected ({})", self.cfg, self.cfg.role);

        let mut items = Vec::with_capacity(128);

        loop {
            match futures::future::select(handle.next(), &mut *shutdown).await {
                Either::Left((Some((message, _)), _)) => {
                    decoder.decode(&message, &mut items);

                    if let Ok(mut stats) = self.monitor.stats.lock() {
                        *stats = decoder.stats().clone();
                    }

                    for item in items.drain(..) {
                        if self.sender.send(Ok(item)).is_err() {
                            return Session::Stopped; // source dropped
                        }
                    }
                }
                Either::Left((None, _)) => {
                    #[cfg(feature = "log")]
                    info!("{}: end of stream", self.cfg);
                    return Session::Ended;
                }
                Either::Right(_) => {
                    #[cfg(feature = "log")]
                    debug!("{}: shutdown", self.cfg);
                    return Session::Stopped;
                }
            }
        }
    }
}

#[cfg(feature = "navigation")]
impl crate::navigation::QcNavigationConfig {
    /// Selects the rover (first [QcNtripRole::Rover] stream) and the RTK base
    /// (first [QcNtripRole::Base] stream) from the roles of these NTRIP streams.
    /// The position of the base is learned from its stream (RTCM 1005/1006)
    /// or from [QcNtripConfig::reference_position_ecef_m].
    pub fn with_ntrip_roles(mut self, streams: &[QcNtripConfig]) -> Self {
        if let Some(rover) = streams.iter().find(|s| s.role == QcNtripRole::Rover) {
            self = self.with_rover(rover.indexing());
        }

        if let Some(base) = streams.iter().find(|s| s.role == QcNtripRole::Base) {
            self = self.with_rtk_base(base.indexing());
        }

        self
    }
}
