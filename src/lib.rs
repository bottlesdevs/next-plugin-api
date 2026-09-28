//! Guest bindings for Bottles plugin interfaces.
//!
//! Build guest crates as `cdylib` with
//! `cargo +nightly-2026-09-25 build --target wasm32-wasip3`.
//! The guest toolchain must be selected explicitly by crates that depend on this SDK.
//! Implement [`Plugin`] and the provider traits, then select the exported
//! providers with `export!(EpicGamesPlugin, account, library)`.
//!
//! The host supplies standard WASI state and constructs one guest `plugin-state.plugin`
//! resource per session. Provider methods borrow that same object as `&self`.
//! Use interior mutability for changing state and release mutable guards before
//! an `.await`. On WASIp3, `thread_local!` storage belongs to each component task.

pub mod account;
pub mod library;

pub use bottles_plugin_macros::export;

/// Constructs one persistent guest object for a plugin session.
pub trait Plugin: Sized + 'static {
    fn new() -> Self;
}

#[doc(hidden)]
pub mod __private {
    pub use wit_bindgen;
}

/// A small buffered client over the standard WASI HTTP interfaces.
pub mod http_client {
    use bytes::Bytes;
    use http_body_util::{BodyExt, Full};

    pub use http::{Method, Request, Response};
    use wasip3::http::client;
    pub use wasip3::http::types::ErrorCode;
    use wasip3::http_compat::{http_from_wasi_response, http_into_wasi_request};

    /// Sends one buffered HTTP request and reads its complete response body.
    pub async fn send(request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, ErrorCode> {
        let request = http_into_wasi_request(request.map(Full::<Bytes>::from))?;
        let response = client::send(request).await?;
        let (parts, body) = http_from_wasi_response(response)?.into_parts();
        let body = body.collect().await?.to_bytes().to_vec();
        Ok(Response::from_parts(parts, body))
    }
}
