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

/// A place in a keyset listing: the sort key of the row a page starts at.
///
/// A cursor says where to read from, never what to read. The order and the
/// filter of a listing travel with the request, so a token stays short, cannot
/// contradict the query it arrives with, and can be linked to and shared.
/// `position: None` is the place before every row, which is the first page.
///
/// Reading a listing at a cursor returns the rows from that place onward, in the
/// listing's order, up to the page size: the row at the place is the first row
/// of the page, and where the page after it starts is the [`Page::next`] the
/// listing hands back. A listing reads in one direction only, so where a reader
/// came from is the reader's own trail of cursors rather than something the
/// listing keeps.
///
/// A listing that sorts on more than one column uses a position type that
/// carries every column, so one place stays one value:
///
/// ```text
/// type KVCursor = Cursor<String>;
///
/// struct IssuePosition {
///     value: i64,
///     id: String,
/// }
/// type IssueCursor = Cursor<IssuePosition>;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cursor<N> {
    pub position: Option<N>,
}

impl<N> Cursor<N> {
    /// Cursor of the first page: no position, reading from the start.
    pub fn first() -> Self {
        Cursor { position: None }
    }

    /// Cursor of the page that starts at `position`.
    pub fn at(position: N) -> Self {
        Cursor {
            position: Some(position),
        }
    }

    /// Encodes the cursor as URL-safe unpadded base64 so it can be placed in a
    /// query string or header without further escaping.
    pub fn encode(&self) -> Result<String>
    where
        N: Serialize,
    {
        let json = serde_json::to_vec(self).context("failed to serialize cursor")?;
        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    /// Decodes a token produced by [`Cursor::encode`].
    pub fn decode(encoded: &str) -> Result<Self>
    where
        N: DeserializeOwned,
    {
        let json = URL_SAFE_NO_PAD
            .decode(encoded)
            .context("cursor is not valid URL-safe base64")?;
        serde_json::from_slice(&json).context("cursor payload is not valid JSON")
    }
}

/// One page of results plus the cursor of the page after it.
///
/// Every list surface (translations, issues, changelog, coverage) returns this
/// shape, so pagination behaves identically everywhere: `next` is `None` on the
/// last page and otherwise names the place the next page starts at.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T, C> {
    pub items: Vec<T>,
    pub next: Option<C>,
}
