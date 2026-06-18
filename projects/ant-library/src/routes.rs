use std::{collections::HashSet, sync::Arc};

use axum::{routing::MethodRouter, Router};
use http::Method;

/// Builder that registers axum routes and tracks their descriptions for a
/// generated fallback handler. Call [`Routes::build`] to get the finished
/// [`Router`] with a `fallback` already attached.
///
/// Sub-routers expose `pub fn routes() -> Routes<S>` without calling `build`.
/// Only the top-level assembly (lib.rs / main.rs / test fixtures) calls `build`.
///
/// ```rust,ignore
/// let router = Routes::new()
///     .merge_routes(version::routes())
///     .nest_routes("/ants", ants::routes())
///     .get("/ping", get(api_ping))
///     .build()
///     .with_state(state);
/// ```
pub struct Routes<S = ()> {
    router: Router<S>,
    descriptions: Vec<(Method, String)>,
    paths: HashSet<&'static str>,
}

impl<S> Routes<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            descriptions: Vec::new(),
            paths: HashSet::new(),
        }
    }

    /// Merge a `Routes` builder into this one. Descriptions are carried over
    /// unchanged.
    pub fn merge_routes(mut self, other: Routes<S>) -> Self {
        self.router = self.router.merge(other.router);
        self.descriptions.extend(other.descriptions);
        self.paths.extend(other.paths);
        self
    }

    /// Nest a `Routes` builder under `path`. Each carried-over description has
    /// its path prefixed with `path`.
    pub fn nest_routes(mut self, path: &'static str, inner: Routes<S>) -> Self {
        self.router = self.router.nest(path, inner.router);
        for (method, route) in inner.descriptions {
            self.descriptions.push((method, format!("{path}{route}")));
        }
        self
    }

    pub fn get(mut self, path: &'static str, method_router: MethodRouter<S>) -> Self {
        self.descriptions.push((Method::GET, path.to_string()));
        self.register(path, method_router)
    }

    pub fn post(mut self, path: &'static str, method_router: MethodRouter<S>) -> Self {
        self.descriptions.push((Method::POST, path.to_string()));
        self.register(path, method_router)
    }

    pub fn put(mut self, path: &'static str, method_router: MethodRouter<S>) -> Self {
        self.descriptions.push((Method::PUT, path.to_string()));
        self.register(path, method_router)
    }

    pub fn head(mut self, path: &'static str, method_router: MethodRouter<S>) -> Self {
        self.descriptions.push((Method::HEAD, path.to_string()));
        self.register(path, method_router)
    }

    pub fn delete(mut self, path: &'static str, method_router: MethodRouter<S>) -> Self {
        self.descriptions.push((Method::DELETE, path.to_string()));
        self.register(path, method_router)
    }

    /// Apply a [`tower::Layer`] to all routes in this builder.
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: tower::Layer<axum::routing::Route> + Clone + Send + Sync + 'static,
        L::Service: tower::Service<
                axum::extract::Request,
                Error = std::convert::Infallible,
            > + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as tower::Service<axum::extract::Request>>::Response:
            axum::response::IntoResponse + 'static,
        <L::Service as tower::Service<axum::extract::Request>>::Future: Send + 'static,
    {
        Self {
            router: self.router.layer(layer),
            descriptions: self.descriptions,
            paths: self.paths,
        }
    }

    /// Consume the builder, attach a generated fallback, and return the
    /// finished [`Router`].
    pub fn build(self) -> Router<S> {
        let mut lines: Vec<String> = self
            .descriptions
            .iter()
            .map(|(method, path)| format!("{method} {path}"))
            .collect();
        lines.sort();
        let descriptions = Arc::new(lines);
        self.router.fallback(move || {
            let d = descriptions.clone();
            async move {
                let strs: Vec<&str> = d.iter().map(|s| s.as_str()).collect();
                crate::api_fallback(&strs)
            }
        })
    }

    // Uses route_with_tsr for the first registration of a path; plain route()
    // for subsequent registrations on the same path to avoid TSR conflicts.
    // Catch-all routes (e.g. `/{*path}`) are registered without TSR: the
    // redirect path `/{*path}/` would put a segment after the wildcard, which
    // axum rejects.
    fn register(mut self, path: &'static str, method_router: MethodRouter<S>) -> Self {
        let is_catch_all = path.contains("{*");
        if is_catch_all || self.paths.contains(path) {
            self.router = self.router.route(path, method_router);
        } else {
            use axum_extra::routing::RouterExt;
            self.router = self.router.route_with_tsr(path, method_router);
        }
        self.paths.insert(path);
        self
    }
}
