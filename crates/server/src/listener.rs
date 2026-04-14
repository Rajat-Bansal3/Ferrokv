use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use config::{Config, ServerConfig};
use persist::persist::PersistHandle;
use storage::Store;
use tokio::net::{TcpListener, TcpStream};

use crate::{connection::Connection, error::ServerError};

pub struct Listener {
    listener: TcpListener,
    store: Arc<dyn Store>,
    config: ServerConfig,
    active_connections: Arc<AtomicUsize>,
    next_conn_id: AtomicU64,
    persist: Arc<PersistHandle>,
}
impl Listener {
    pub async fn new(config: Config, store: Arc<dyn Store>) -> Result<Self, ServerError> {
        let listner =
            tokio::net::TcpListener::bind(format!("{}:{}", config.server.host, config.server.port))
                .await
                .map_err(|_| ServerError::ErrorInitialisingLister)?;
        let persist_handler = PersistHandle::new(&config.persistence)
            .map_err(|_| ServerError::ErrorInitilisingPersistance)?;
        Ok(Listener {
            listener: listner,
            store: store,
            config: config.server,
            active_connections: Arc::new(AtomicUsize::new(0)),
            next_conn_id: AtomicU64::new(0),
            persist: Arc::new(persist_handler),
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
        let max_ideal_time = self.config.max_ideal_time;
        let persist_handler = self.persist.clone();
        tokio::spawn(async move {
            let mut connection = Connection::new(stream, task_store, id);
            let _ = connection.run(max_ideal_time, &persist_handler).await;
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
