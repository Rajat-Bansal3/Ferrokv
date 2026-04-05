use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use config::ServerConfig;
use storage::Store;
use tokio::net::{TcpListener, TcpStream};

use crate::{connection::Connection, error::ServerError};

pub struct Listener {
    listener: TcpListener,
    store: Arc<dyn Store>,
    config: ServerConfig,
    active_connections: Arc<AtomicUsize>,
    next_conn_id: AtomicU64,
}
impl Listener {
    pub async fn new(config: ServerConfig, store: Arc<dyn Store>) -> Result<Self, ServerError> {
        println!("{}:{}", config.host, config.port);
        let listner = tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port))
            .await
            .map_err(|_| ServerError::ErrorInitialisingLister)?;
        Ok(Listener {
            listener: listner,
            store: store,
            config: config,
            active_connections: Arc::new(AtomicUsize::new(0)),
            next_conn_id: AtomicU64::new(0),
        })
    }
    pub async fn run(&self) -> Result<(), ServerError> {
        loop {
            let (stream, _) = self
                .listener
                .accept()
                .await
                .map_err(|_| ServerError::ErrorAcceptingConnections)?;
            if !self.is_connection_allowed() {
                drop(stream);
                continue;
            }
            self.handle_connection(stream, self.next_id());
        }
    }
    fn handle_connection(&self, stream: TcpStream, id: u64) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        let task_store = self.store.clone();
        let task_active_connections = self.active_connections.clone();
        tokio::spawn(async move {
            let mut connection = Connection::new(stream, task_store, id);
            connection.run().await;
            task_active_connections.fetch_sub(1, Ordering::Relaxed);
        });
    }
    fn is_connection_allowed(&self) -> bool {
        if self.config.max_connections == 0 {
            return true;
        }
        self.active_connections.as_ref().load(Ordering::Relaxed) < self.config.max_connections
    }
    fn next_id(&self) -> u64 {
        self.next_conn_id.fetch_add(1, Ordering::Relaxed)
    }
}
