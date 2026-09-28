use std::fmt::{self, Display};

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Default number of items returned for a page when the caller does not ask for a size.
pub const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortOrder::Asc => f.write_str("ASC"),
            SortOrder::Desc => f.write_str("DESC"),
        }
    }
}

/// Keyset cursor for one sort key plus its tiebreaker.
///
/// `K` is the per-table closed set of supported sort keys, so each table gets
/// its own concrete cursor type:
///
/// ```text
/// enum ChangelogKey {
///     Id { id: i64 },
///     CreatedAt { created_at: String, id: i64 },
/// }
/// type ChangelogCursor = QueryCursor<ChangelogKey>;
/// ```
///
/// Because the set is closed, the variant already identifies the query and its
/// ordering, so no ordering whitelist or query fingerprint is needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCursor<K> {
    pub order: SortOrder,
    pub key: K,
}

impl<K> QueryCursor<K> {
    pub fn new(order: SortOrder, key: K) -> Self {
        QueryCursor { order, key }
    }

    /// Encodes the cursor as URL-safe unpadded base64 so it can be placed in a
    /// query string or header without further escaping.
    pub fn encode(&self) -> Result<String>
    where
        K: Serialize,
    {
        let json = serde_json::to_vec(self).context("failed to serialize cursor")?;
        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    /// Decodes a token produced by [`QueryCursor::encode`].
    pub fn decode(encoded: &str) -> Result<Self>
    where
        K: DeserializeOwned,
    {
        let json = URL_SAFE_NO_PAD
            .decode(encoded)
            .context("cursor is not valid URL-safe base64")?;
        serde_json::from_slice(&json).context("cursor payload is not valid JSON")
    }
}
