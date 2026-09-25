//! Guest bindings for Bottles plugin interfaces.
//!
//! Build guest crates as `cdylib` with
//! `cargo +nightly-2026-09-25 build --target wasm32-wasip3`.
//! The guest toolchain must be selected explicitly by crates that depend on this SDK.
//! Implement the selected interfaces with async functions and export them with
//! `export!(Plugin, account, library)` or just the interfaces the plugin provides.
//!
//! A host session preserves guest memory between calls. On WASIp3, `thread_local!`
//! storage belongs to each component task; use ordinary static storage for state
//! that must survive separate calls in the same session.

/// Account linking through an explicit host interaction capability.
pub mod account {
    wit_bindgen::generate!({ path: "wit", world: "account", pub_export_macro: true });
    pub use bottles::plugin::account_link::Interaction;
    pub use exports::bottles::plugin::account_provider::*;
}

/// Installed, launchable titles supplied by a plugin.
///
/// Entry IDs belong to the provider. Launch completion reports that the request
/// finished, not that the title exited. Local filesystem and process capabilities
/// are not supplied by this interface.
pub mod library {
    wit_bindgen::generate!({ path: "wit", world: "library", pub_export_macro: true });
    pub use exports::bottles::plugin::library_provider::*;
}

/// Export the selected SDK interfaces implemented by a plugin.
#[macro_export]
macro_rules! export {
    ($plugin:ident, $($integration:ident),+ $(,)?) => {
        $(
            $crate::$integration::export!(
                $plugin with_types_in $crate::$integration
            );
        )+
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
