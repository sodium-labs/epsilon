use crate::environment::Environment;
use axum::Router;
use routes::{
    analytics::create_analytics_router, base::create_base_router,
    statistics::create_statistics_router, votes::create_votes_router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::{Config, SwaggerUi};

pub mod environment;
mod routes;

#[derive(OpenApi)]
#[openapi()]
struct ApiDoc;

pub async fn build_api(env: Arc<Environment>, port: u16) {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", create_base_router())
        .nest("/api/statistics", create_statistics_router())
        .nest("/api/analytics", create_analytics_router())
        .nest("/api/votes", create_votes_router())
        .with_state(env)
        .split_for_parts();

    let router = router.merge(
        SwaggerUi::new("/docs")
            .config(Config::default())
            .url("/docs/openapi.json", api.clone()),
    );

    let app = Router::new().merge(router);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("[API] API listening on {addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
