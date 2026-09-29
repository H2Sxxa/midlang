/// Liveness probe for the HTTP service.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is alive", body = String)
    )
)]
pub async fn health() -> &'static str {
    "OK"
}
