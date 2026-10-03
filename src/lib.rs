//! Guest bindings for Bottles plugin interfaces.
//!
//! Build guest crates as `cdylib` with
//! `cargo +nightly-2026-09-25 build --target wasm32-wasip3`.
//! The guest toolchain must be selected explicitly by crates that depend on this SDK.
//! Implement [`Plugin`] and the provider traits, then select the exported
//! providers with `export!(EpicGamesPlugin: account, library)`.
//!
//! One instance-global value is shared across capabilities. Calls can run concurrently:
//! use interior mutability and release mutex guards before `.await`.

pub mod account;
pub mod library;

/// Constructs the guest state shared by all exported capabilities.
pub trait Plugin: Send + Sync + Sized + 'static {
    fn new() -> Self;
}

/// Exports one or more capability worlds from one guest instance.
#[macro_export]
macro_rules! export {
    ($plugin:ident: $($cap:ident),+ $(,)?) => {
        static __BOTTLES_PLUGIN: ::std::sync::OnceLock<$plugin> = ::std::sync::OnceLock::new();
        $( $crate::$cap::__glue!($plugin, __BOTTLES_PLUGIN); )+
    };
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
