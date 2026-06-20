//! Local-only HTTP JSON API (127.0.0.1). Not MCP — no OAuth, no remote exposure.

#[cfg(feature = "local-http")]
mod imp {
    use axum::{
        extract::{Query, State},
        routing::{get, post},
        Json, Router,
    };
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::{gather_context, BrainEngine, HashEmbedder};

    #[derive(Clone)]
    struct AppState {
        engine: Arc<Mutex<BrainEngine>>,
    }

    #[derive(Deserialize)]
    struct SearchQ {
        q: String,
        limit: Option<usize>,
    }

    #[derive(Deserialize)]
    struct PutBody {
        slug: String,
        title: Option<String>,
        body: String,
        page_type: Option<String>,
    }

    #[derive(Serialize)]
    struct Health {
        ok: bool,
        local_only: bool,
    }

    pub async fn serve(bind: SocketAddr, engine: BrainEngine) -> anyhow::Result<()> {
        let state = AppState {
            engine: Arc::new(Mutex::new(engine)),
        };
        let app = Router::new()
            .route("/health", get(health))
            .route("/search", get(search))
            .route("/query", get(query))
            .route("/put", post(put_page))
            .route("/dream", post(dream))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind(bind).await?;
        tracing::info!("rs_gbrain local HTTP on {bind} (not MCP)");
        axum::serve(listener, app).await?;
        Ok(())
    }

    async fn health() -> Json<Health> {
        Json(Health {
            ok: true,
            local_only: true,
        })
    }

    async fn search(
        State(st): State<AppState>,
        Query(q): Query<SearchQ>,
    ) -> Json<serde_json::Value> {
        let guard = st.engine.lock().await;
        match guard.search(&q.q, q.limit.unwrap_or(10)) {
            Ok(h) => Json(serde_json::json!({ "hits": h })),
            Err(e) => Json(serde_json::json!({ "error": format!("{e:#}") })),
        }
    }

    async fn query(
        State(st): State<AppState>,
        Query(q): Query<SearchQ>,
    ) -> Json<serde_json::Value> {
        let guard = st.engine.lock().await;
        match gather_context(&guard, &q.q, q.limit.unwrap_or(8)) {
            Ok(a) => Json(serde_json::to_value(a).unwrap_or_default()),
            Err(e) => Json(serde_json::json!({ "error": format!("{e:#}") })),
        }
    }

    async fn put_page(
        State(st): State<AppState>,
        Json(body): Json<PutBody>,
    ) -> Json<serde_json::Value> {
        let guard = st.engine.lock().await;
        let title = body.title.unwrap_or_else(|| {
            body.slug
                .rsplit('/')
                .next()
                .unwrap_or(&body.slug)
                .to_string()
        });
        let pt = body.page_type.unwrap_or_else(|| "note".to_string());
        match guard.put_page(&body.slug, &title, &pt, &body.body, "http") {
            Ok(()) => Json(serde_json::json!({ "ok": true, "slug": body.slug })),
            Err(e) => Json(serde_json::json!({ "error": format!("{e:#}") })),
        }
    }

    async fn dream(State(st): State<AppState>) -> Json<serde_json::Value> {
        let guard = st.engine.lock().await;
        match crate::run_nightly_cycle(&guard, &HashEmbedder) {
            Ok(r) => Json(serde_json::to_value(r).unwrap_or_default()),
            Err(e) => Json(serde_json::json!({ "error": format!("{e:#}") })),
        }
    }
}

#[cfg(feature = "local-http")]
pub use imp::serve;

#[cfg(not(feature = "local-http"))]
pub async fn serve(_bind: std::net::SocketAddr, _engine: crate::BrainEngine) -> anyhow::Result<()> {
    anyhow::bail!("rebuild with --features local-http for local JSON API (not MCP)")
}
