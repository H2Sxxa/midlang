use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};

pub enum SortOrder {
    Asc,
    Desc,
}

pub const DEFAULT_PAGE_SIZE: usize = 100;

impl ToString for SortOrder {
    fn to_string(&self) -> String {
        match self {
            SortOrder::Asc => "ASC".to_string(),
            SortOrder::Desc => "DESC".to_string(),
        }
    }
}

// Require data to be serializable and deserializable for Cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCursor<D> {
    pub data: D,
}

impl<D> QueryCursor<D> {
    pub fn new(data: D) -> Self {
        QueryCursor { data }
    }

    pub fn encode(&self) -> Result<String>
    where
        D: Serialize,
    {
        let json = serde_json::to_string(&self)?;
        Ok(BASE64_STANDARD.encode(json))
    }

    pub fn decode(encoded: &str) -> Result<D>
    where
        D: for<'de> Deserialize<'de>,
    {
        let json = BASE64_STANDARD.decode(encoded)?;
        let cursor: QueryCursor<D> = serde_json::from_slice(&json)?;
        Ok(cursor.data)
    }
}
