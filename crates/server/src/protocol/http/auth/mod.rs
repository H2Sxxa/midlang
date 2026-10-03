pub mod groups;
pub mod permissions;
pub mod tokens;

use axum::Router;

pub fn router<State>() -> Router<State>
where
    State: Clone + Send + Sync + 'static,
{
    permissions::router::<State>()
        .merge(groups::router::<State>())
        .merge(tokens::router::<State>())
}
