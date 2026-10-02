pub mod statistics;

use axum::{Router, routing::get};
use midlang_core::translation::Translation;

pub fn router<Store>() -> Router<Translation<Store>>
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    Router::new().route(
        "/store/statistics",
        get(statistics::statistics_handler::<Store>),
    )
}
