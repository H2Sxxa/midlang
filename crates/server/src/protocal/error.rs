use anyhow::Error;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use midlang_core::store::StoreError;
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

/// The store does not hold this locale or key. It is a normal answer for a
/// lookup, not a failure, so it is separated from [`store`].
pub fn not_found(message: String) -> Response {
    (StatusCode::NOT_FOUND, Json(ErrorBody { error: message })).into_response()
}

/// Maps a store failure to a distinct response: a missing locale or key is a
/// `404`, everything else is a `500` logged with its full error chain.
pub fn store(err: Error) -> Response {
    match err.downcast_ref::<StoreError>() {
        Some(StoreError::LocaleNotExist(_)) | Some(StoreError::LocaleKeyNotExist(_, _)) => {
            not_found(err.to_string())
        }
        _ => {
            tracing::error!(error = ?err, "translation store request failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal error".to_string(),
                }),
            )
                .into_response()
        }
    }
}
