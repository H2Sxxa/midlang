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
        super::auth::permissions::permissions_handler,
        super::auth::tokens::list_tokens_handler,
        super::auth::tokens::create_token_handler,
        super::auth::tokens::revoke_token_handler,
        super::auth::tokens::rotate_token_handler,
        super::store::statistics::statistics_handler,
        super::translate::locale::list_translation_handler,
        super::translate::key::translate_handler,
        super::translate::key::set_translation_handler,
        super::translate::key::delete_translation_handler,
        super::translate::namespace::translate_namespace_handler,
        super::translate::namespace::set_namespace_translation_handler,
        super::translate::namespace::delete_namespace_translation_handler
    ),
    components(
        schemas(
            super::translate::SetTranslationRequest,
            super::translate::TranslationMessage,
            super::translate::locale::TranslationListEntry,
            super::translate::locale::TranslationListPage,
            super::store::statistics::StoreStatistics,
            super::auth::permissions::PermissionsResponse,
            super::auth::tokens::CreateTokenRequest,
            super::auth::tokens::TokenListResponse,
            crate::secure::CreatedToken,
            crate::secure::TokenInfo,
            crate::protocol::error::ErrorBody
        )
    )
)]
pub struct ApiDoc;

impl ApiDoc {
    pub fn document() -> utoipa::openapi::OpenApi {
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
    Json(ApiDoc::document())
}
