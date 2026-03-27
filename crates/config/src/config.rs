use anyhow::Ok;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FsyncPolicy {
    Always,
    EverySec,
    No,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EvictionPolicy {
    NoEviction,
    AllKeysLru,
    VolatileLru,
    AllKeysRandom,
    VolatileRandom,
    AllKeysLfu,
    VolatileLfu,
    VolatileTtl,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tcp_backlog: u32,
    pub max_connections: usize,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageConfig {
    pub maxmemory: usize,
    pub shard_count: usize,
    pub eviction_policy: EvictionPolicy,
    pub ttl_sweep_interval_ms: u64,
    pub eviction_sample_size: usize,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistenceConfig {
    pub aof_enabled: bool,
    pub rdb_enabled: bool,
    pub fsync: FsyncPolicy,
    pub aof_path: String,
    pub rdb_path: String,
    pub rdb_save_interval_secs: u64,
    pub aof_rewrite_min_size: usize,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub json: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub persistence: PersistenceConfig,
    pub logging: LoggingConfig,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            persistence: PersistenceConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}
impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 6379,
            tcp_backlog: 511,
            max_connections: 10_000,
        }
    }
}
impl Default for EvictionPolicy {
    fn default() -> Self {
        EvictionPolicy::NoEviction
    }
}
impl Default for FsyncPolicy {
    fn default() -> Self {
        FsyncPolicy::No
    }
}
impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            maxmemory: 0,
            shard_count: num_cpus::get() * 4,
            eviction_policy: EvictionPolicy::default(),
            ttl_sweep_interval_ms: 100,
            eviction_sample_size: 5,
        }
    }
}
impl Default for PersistenceConfig {
    fn default() -> Self {
        PersistenceConfig {
            aof_enabled: false,
            rdb_enabled: false,
            fsync: FsyncPolicy::default(),
            aof_path: "./ferrokv.aof".to_string(),
            rdb_path: "./ferrokv.rdb".to_string(),
            rdb_save_interval_secs: 300,
            aof_rewrite_min_size: 64 * 1024 * 1024,
        }
    }
}
impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: "info".to_string(),
            json: false,
        }
    }
}
impl Config {
    pub fn builder(path: Option<&str>) -> anyhow::Result<()> {
        let default_config_toml = toml::to_string_pretty(&Self::default())
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

        let target = path.unwrap_or("./ferrokv.config.toml");
        std::fs::write(target, default_config_toml)?;
        Ok(())
    }
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let config = match path {
            None => Config::default(),
            Some(p) => {
                let contents = std::fs::read_to_string(p)?;
                let config: Config = toml::from_str(&contents)?;
                config
            }
        };
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.server.port == 0 {
            anyhow::bail!("port 0 ?");
        }
        if self.storage.shard_count == 0 {
            anyhow::bail!("shard_count 0 ?");
        }
        if self.storage.shard_count & (self.storage.shard_count - 1) != 0 {
            anyhow::bail!("shard_count shoudl be a power of 2");
        }
        if self.storage.eviction_sample_size == 0 {
            anyhow::bail!("eviction_sample_size 0 ?");
        }
        Ok(())
    }
}
