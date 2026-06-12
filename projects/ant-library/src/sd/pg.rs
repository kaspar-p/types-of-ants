use bb8::ManageConnection;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_postgres::{Client, NoTls};
use tracing::debug;

use crate::sd::reader::{ServiceDiscovery, ServiceEndpoint};

pub struct TrackedConnection {
    _handle: JoinHandle<Result<(), tokio_postgres::Error>>,
    conn: Client,
    host: String,
    pub port: u16,
}

impl Deref for TrackedConnection {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl DerefMut for TrackedConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

enum ServiceRef {
    Static {
        host: String,
        port: u16,
    },
    Dynamic {
        sd: Arc<ServiceDiscovery>,
        service: String,
    },
}

impl ServiceRef {
    async fn resolve(&self) -> Option<ServiceEndpoint> {
        match self {
            ServiceRef::Static { host, port } => Some(ServiceEndpoint {
                node: host.clone(),
                address: host.clone(),
                port: *port,
            }),
            ServiceRef::Dynamic { sd, service } => sd.resolve(service).await,
        }
    }
}

pub struct PostgresManager {
    source: ServiceRef,
    dbname: String,
    user: String,
    password: String,
}

impl PostgresManager {
    /// Connect to a fixed host:port — the endpoint never changes.
    pub fn new_static(
        host: impl Into<String>,
        port: u16,
        dbname: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            source: ServiceRef::Static {
                host: host.into(),
                port,
            },
            dbname: dbname.into(),
            user: user.into(),
            password: password.into(),
        }
    }

    /// Connect via Consul service discovery — endpoint is re-resolved on every
    /// new connection and recycled when it changes.
    pub fn new_dynamic(
        sd: Arc<ServiceDiscovery>,
        service: impl Into<String>,
        dbname: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            source: ServiceRef::Dynamic {
                sd,
                service: service.into(),
            },
            dbname: dbname.into(),
            user: user.into(),
            password: password.into(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PoolError {
    #[error("service not found: {0}")]
    ServiceNotFound(String),

    #[error("connection failed: {0:?}")]
    ConnectFailed(tokio_postgres::Error),

    #[error("endpoint changed")]
    EndpointChanged,

    #[error("connection closed")]
    ConnectionClosed,
}

impl From<tokio_postgres::Error> for PoolError {
    fn from(e: tokio_postgres::Error) -> Self {
        Self::ConnectFailed(e)
    }
}

pub fn make_connection_string(
    username: &str,
    password: &str,
    host: &str,
    port: u16,
    db_name: &str,
) -> String {
    format!("postgresql://{username}:{password}@{host}:{port}/{db_name}")
}

impl ManageConnection for PostgresManager {
    type Connection = TrackedConnection;
    type Error = PoolError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let ep = self.source.resolve().await.ok_or_else(|| {
            PoolError::ServiceNotFound(match &self.source {
                ServiceRef::Static { host, .. } => host.clone(),
                ServiceRef::Dynamic { service, .. } => service.clone(),
            })
        })?;

        let mut config = tokio_postgres::Config::new();
        config.host(&ep.address);
        config.port(ep.port);
        config.dbname(&self.dbname);
        config.user(&self.user);
        config.password(&self.password);

        debug!(
            "Connecting to database: {}",
            make_connection_string(&self.user, "[redacted]", &ep.address, ep.port, &self.dbname)
        );

        let (client, connection) = config.connect(NoTls).await?;
        let handle = tokio::spawn(connection);

        Ok(TrackedConnection {
            _handle: handle,
            conn: client,
            host: ep.address,
            port: ep.port,
        })
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        debug!("Checking connection validity.");
        if conn.conn.is_closed() {
            return Err(PoolError::ConnectionClosed);
        }
        // Static connections never change endpoint; skip the resolve.
        if let ServiceRef::Static { .. } = &self.source {
            return Ok(());
        }
        match self.source.resolve().await {
            Some(ref ep) if ep.address == conn.host && ep.port == conn.port => Ok(()),
            _ => Err(PoolError::EndpointChanged),
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.conn.is_closed()
    }
}
