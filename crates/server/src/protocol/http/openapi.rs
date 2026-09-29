use axum::Json;
use utoipa::OpenApi;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "MidLang API",
        version = env!("CARGO_PKG_VERSION"),
        description = "Translation service API"
    ),
    paths(
        super::health::health,
        super::store::statistics_handler,
        super::translate::translate_handler,
        super::translate::set_translation_handler,
        super::translate::delete_translation_handler,
        super::translate::translate_namespace_handler,
        super::translate::set_namespace_translation_handler,
        super::translate::delete_namespace_translation_handler
    ),
    components(
        schemas(
            super::translate::SetTranslationRequest,
            super::translate::TranslationMessage,
            super::store::StoreStatistics,
            crate::protocol::error::ErrorBody
        )
    )
)]
pub struct ApiDoc;

impl ApiDoc {
    fn with_security() -> utoipa::openapi::OpenApi {
        let mut document = <Self as OpenApi>::openapi();
        document
            .components
            .get_or_insert_with(utoipa::openapi::Components::new)
            .add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        document
    }
}

pub async fn handler() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::with_security())
}
