//! A place in a keyset listing, spelled the way SQL compares it.

use crate::query::SortOrder;

/// The comparison that reads a listing from a place onward, with the row at the
/// place itself included: `>=` ascending, `<=` descending.
pub fn inclusive(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Asc => ">=",
        SortOrder::Desc => "<=",
    }
}

/// The comparison that reads a listing past a place, with the row at the place
/// itself left out: `>` ascending, `<` descending.
pub fn exclusive(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Asc => ">",
        SortOrder::Desc => "<",
    }
}
