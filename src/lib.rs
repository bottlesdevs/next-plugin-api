#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod account;
pub mod library;

/// Implemented by `export!`; do not implement manually.
///
/// The macro uses [`Default`] to construct one instance shared by all selected
/// capabilities. Provider methods receive references to that instance. Mutable
/// state needs synchronization because calls may run concurrently.
///
/// # Examples
///
/// ```text
/// #[derive(Default)]
/// struct MyPlugin;
/// // Implement the library provider trait for MyPlugin, then:
/// bottles_plugin_api::export!(MyPlugin: library);
/// ```
pub trait Plugin: Default + Send + Sync + 'static {
    /// Returns the shared guest state, initializing it on first use.
    ///
    /// The generated implementation uses [`Default::default()`] to initialize
    /// state; subsequent calls reuse it. If construction panics, the instance
    /// remains uninitialized.
    ///
    /// # Panics
    ///
    /// Panics if the plugin's [`Default`] implementation panics.
    ///
    /// # Examples
    ///
    /// ```text
    /// use bottles_plugin_api::Plugin;
    /// let state = MyPlugin::instance();
    /// ```
    fn instance() -> &'static Self;
}

/// Exports one or more capability worlds from one guest instance.
///
/// The type must implement [`Default`] and the provider trait for each listed
/// capability: [`account::AccountProvider`] for `account`, or
/// [`library::LibraryProvider`] for `library`. It must also be [`Send`], [`Sync`],
/// and `'static`. The macro implements [`Plugin`] and initializes its state
/// lazily through [`Default::default()`].
///
/// Invoke this macro once for the type. Only listed capabilities are exported;
/// implementing another provider trait does not add an export.
///
/// # Examples
///
/// ```text
/// // MyPlugin implements Default and both provider traits.
/// bottles_plugin_api::export!(MyPlugin: account, library);
/// ```
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
///
/// [`http_client::send`] accepts an [`http::Request`] with a [`Vec<u8>`] body and
/// collects the complete response body into another vector. Request and response
/// bodies are held in memory; use it for requests whose bodies fit in guest memory.
///
/// # Examples
///
/// ```text
/// use bottles_plugin_api::http_client::{Request, send};
/// let request = Request::get("https://example.com/catalog")
///     .body(Vec::new())
///     .map_err(|error| error.to_string())?;
/// let response = send(request).await.map_err(|error| error.to_string())?;
/// let bytes = response.into_body();
/// ```
pub mod http_client {
    use bytes::Bytes;
    use http_body_util::{BodyExt, Full};

    /// An HTTP request method, such as GET or POST.
    ///
    /// # Examples
    ///
    /// ```text
    /// use bottles_plugin_api::http_client::Method;
    /// let method = Method::POST;
    /// ```
    pub use http::Method;
    /// An HTTP request with a generic body; [`send`] accepts `Request<Vec<u8>>`.
    ///
    /// # Examples
    ///
    /// ```text
    /// use bottles_plugin_api::http_client::Request;
    /// let request = Request::get("https://example.com/catalog").body(Vec::new())?;
    /// ```
    pub use http::Request;
    /// An HTTP response with status, headers, and a generic body.
    ///
    /// # Examples
    ///
    /// ```text
    /// let status = response.status();
    /// let body = response.into_body();
    /// ```
    pub use http::Response;
    use wasip3::http::client;
    /// A WASI HTTP failure during conversion, sending, or body collection.
    ///
    /// # Examples
    ///
    /// ```text
    /// let response = bottles_plugin_api::http_client::send(request)
    ///     .await
    ///     .map_err(|error| error.to_string())?;
    /// ```
    pub use wasip3::http::types::ErrorCode;
    use wasip3::http_compat::{http_from_wasi_response, http_into_wasi_request};

    /// Sends one buffered HTTP request and reads its complete response body.
    ///
    /// Non-success HTTP statuses remain ordinary responses; inspect the status
    /// when deciding whether the provider's request succeeded.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode`] if request or response conversion fails, the WASI
    /// HTTP client cannot send the request, or response-body collection fails.
    ///
    /// # Examples
    ///
    /// ```text
    /// use bottles_plugin_api::http_client::{Request, send};
    /// let request = Request::get("https://example.com/catalog")
    ///     .body(Vec::new())
    ///     .map_err(|error| error.to_string())?;
    /// let response = send(request).await.map_err(|error| error.to_string())?;
    /// ```
    pub async fn send(request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, ErrorCode> {
        let request = http_into_wasi_request(request.map(Full::<Bytes>::from))?;
        let response = client::send(request).await?;
        let (parts, body) = http_from_wasi_response(response)?.into_parts();
        let body = body.collect().await?.to_bytes().to_vec();
        Ok(Response::from_parts(parts, body))
    }
}
