use clap::{Parser, builder::ValueParser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The address to bind the server to.
    #[arg(short, long, default_value_t = 4321)]
    pub port: u16,
    /// The database type to use for the translation store. Currently, only Redb is supported.
    #[arg(default_value_t = StoreType::default(), value_parser = ValueParser::from(StoreType::from_str), long)]
    pub store: StoreType,

    /// The URL of the database to use.
    #[arg(long)]
    pub store_url: Option<String>,

    /// The capacity of the lru cache.
    #[arg(long, default_value_t = 10000)]
    pub cache_capacity: usize,

    /// The URL of the SQLite database to use. This is only used when the database type is set to SQLite.
    #[arg(long)]
    pub sqlite_url: Option<String>,
    /// Whether to enable issue tracking.
    #[arg(long, default_value_t = true)]
    pub issue: bool,
    /// Whether to enable changelog.
    #[arg(long, default_value_t = true)]
    pub changelog: bool,
}

#[derive(Debug, Clone, Default)]
pub enum StoreType {
    #[default]
    Redb,
}

impl ToString for StoreType {
    fn to_string(&self) -> String {
        match self {
            StoreType::Redb => "redb".into(),
        }
    }
}

impl StoreType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "redb" => Ok(StoreType::Redb),
            _ => Err(format!("Invalid store type: {}", s)),
        }
    }
}
