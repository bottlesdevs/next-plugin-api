//! Guest bindings for Bottles plugin interfaces.
//!
//! Build guest crates as `cdylib` with
//! `cargo +nightly-2026-09-25 build --target wasm32-wasip3`.
//! The guest toolchain must be selected explicitly by crates that depend on this SDK.
//! Implement [`Default`] and the provider traits, then select the exported
//! providers with `export!(EpicGamesPlugin: account, library)`.
//!
//! `export!` creates the shared instance with `Default::default()` on first use.
//! Calls may run concurrently:
//! use interior mutability and release mutex guards before `.await`.

pub mod account;
pub mod library;

/// Implemented by `export!`; do not implement manually.
pub trait Plugin: Default + Send + Sync + 'static {
    fn instance() -> &'static Self;
}

/// Exports one or more capability worlds from one guest instance.
#[macro_export]
macro_rules! export {
    ($plugin:ident: $($cap:ident),+ $(,)?) => {
        impl $crate::Plugin for $plugin {
            fn instance() -> &'static Self {
                static INSTANCE: ::std::sync::OnceLock<$plugin> = ::std::sync::OnceLock::new();
                INSTANCE.get_or_init(<$plugin as ::core::default::Default>::default)
            }
        }
        $( $crate::$cap::__bindings::export!($plugin with_types_in $crate::$cap::__bindings); )+
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
