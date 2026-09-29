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

/// Keyset cursor that carries everything a page needs: the ordering, the filter
/// and the position to resume from.
///
/// `K` is the filter (which rows are in scope, e.g. a keyword) and `N` is the
/// position (the sort key of the last row that was returned, e.g. the entry
/// key). Page one is [`QueryCursor::new`] with `next: None`; every following
/// page passes back the cursor of the previous page, so the ordering and filter
/// cannot drift away from the position.
///
/// A table that sorts on a single unique key uses that key directly, a table
/// that sorts on more than one column uses a closed enum:
///
/// ```text
/// type KVCursor = QueryCursor<KVFilter, String>;
///
/// enum ChangelogKey {
///     Id { id: i64 },
///     CreatedAt { created_at: String, id: i64 },
/// }
/// type ChangelogCursor = QueryCursor<ChangelogFilter, ChangelogKey>;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCursor<K, N> {
    pub order: SortOrder,
    pub filter: K,
    pub next: Option<N>,
}

impl<K, N> QueryCursor<K, N> {
    pub fn new(order: SortOrder, filter: K) -> Self {
        QueryCursor {
            order,
            filter,
            next: None,
        }
    }

    /// Encodes the cursor as URL-safe unpadded base64 so it can be placed in a
    /// query string or header without further escaping.
    pub fn encode(&self) -> Result<String>
    where
        K: Serialize,
        N: Serialize,
    {
        let json = serde_json::to_vec(self).context("failed to serialize cursor")?;
        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    /// Decodes a token produced by [`QueryCursor::encode`].
    pub fn decode(encoded: &str) -> Result<Self>
    where
        K: DeserializeOwned,
        N: DeserializeOwned,
    {
        let json = URL_SAFE_NO_PAD
            .decode(encoded)
            .context("cursor is not valid URL-safe base64")?;
        serde_json::from_slice(&json).context("cursor payload is not valid JSON")
    }
}

/// One page of results plus the cursor that resumes the listing.
///
/// Every list surface (translations, issues, changelog) returns this shape, so
/// pagination behaves identically everywhere: `next` is `None` on the last page
/// and otherwise carries the ordering, the filter and the position, which keeps
/// the resumed query from drifting away from the one that produced the page.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T, C> {
    pub items: Vec<T>,
    pub next: Option<C>,
}
