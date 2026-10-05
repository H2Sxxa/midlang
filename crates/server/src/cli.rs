use clap::{Parser, builder::ValueParser};
use std::{fmt, str::FromStr};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The host address to bind the server to. Use 0.0.0.0 to accept
    /// connections from outside the local machine (for example in a container).
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// The port to bind the server to.
    #[arg(short, long, default_value_t = 4321)]
    pub port: u16,
    /// Allowed CORS origins. Repeat the option or separate origins with commas.
    /// If omitted, all origins are allowed for local development.
    #[arg(long, value_delimiter = ',')]
    pub cors_origin: Vec<String>,
    /// The store type to use for the translation store.
    #[arg(default_value_t = StoreType::default(), value_parser = ValueParser::from(StoreType::from_str), long)]
    pub store: StoreType,

    /// The URL of the Redb database to use. Ignored for the memory store.
    #[arg(long)]
    pub store_url: Option<String>,

    /// The shared SQLite database URL for auth, issue, changelog, and coverage.
    #[arg(long)]
    pub sqlite_url: Option<String>,
    /// Legacy fallback URL when --sqlite-url is not provided. The auth store now
    /// shares the same pool as the internal services.
    #[arg(long)]
    pub auth_sqlite_url: Option<String>,
    /// Whether to enable issue tracking.
    #[arg(long, default_value_t = true)]
    pub issue: bool,
    /// Whether to enable changelog.
    #[arg(long, default_value_t = true)]
    pub changelog: bool,
    /// Whether to maintain per-locale translation coverage in SQLite. Costs one
    /// scan of the store at startup; writes afterwards are incremental.
    #[arg(long, default_value_t = false)]
    pub coverage: bool,
}

#[derive(Debug, Clone, Default)]
pub enum StoreType {
    #[default]
    Redb,
    Memory,
}

impl fmt::Display for StoreType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreType::Redb => write!(f, "redb"),
            StoreType::Memory => write!(f, "memory"),
        }
    }
}

impl FromStr for StoreType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "redb" => Ok(StoreType::Redb),
            "memory" | "mem" => Ok(StoreType::Memory),
            _ => Err(format!(
                "Invalid store type: {} (expected redb or memory)",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StoreType;
    use std::str::FromStr;

    #[test]
    fn parses_both_store_types() {
        assert!(matches!(StoreType::from_str("redb"), Ok(StoreType::Redb)));
        assert!(matches!(
            StoreType::from_str("memory"),
            Ok(StoreType::Memory)
        ));
        assert!(matches!(StoreType::from_str("mem"), Ok(StoreType::Memory)));
    }

    #[test]
    fn rejects_unknown_store_type() {
        assert!(StoreType::from_str("sqlite").is_err());
    }
}
