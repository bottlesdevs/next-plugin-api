//! Rust guest API for Bottles plugins.

pub use __wit::bottles::plugin::account_link::Interaction as AccountLinkInteraction;
pub use __wit::exports::bottles::plugin::storefront_account_provider::{
    AccountIdentity, LinkedAccount,
};
pub use __wit::exports::bottles::plugin::storefront_library_provider::{ListedGames, OwnedGame};

/// Shared state owned by one loaded plugin instance.
pub trait Plugin: Sized + 'static {
    /// Creates the plugin state.
    fn new() -> Result<Self, String>;
}

/// Links storefront accounts.
pub trait StorefrontAccountProvider: Plugin {
    fn link_account(
        &mut self,
        interaction: &AccountLinkInteraction,
    ) -> Result<LinkedAccount, String>;
}

/// Lists games owned through a storefront account.
///
/// Outer errors carry no credential update. Once authentication refreshes,
/// return `Ok(ListedGames)` with the replacement credential, putting any later
/// enumeration error in `games`. `updated_credential: None` retains the current
/// credential. Hosts process the update before reporting an enumeration error.
/// Traps and abandoned calls cannot deliver an update through this result.
pub trait StorefrontLibraryProvider: Plugin {
    fn list_games(
        &mut self,
        account_id: &str,
        credential: Option<&[u8]>,
    ) -> Result<ListedGames, String>;
}

/// Exports a Bottles plugin component.
///
/// A plugin has one shared state resource and declares its implemented
/// capabilities at compile time.
#[macro_export]
macro_rules! export_plugin {
    (@kind StorefrontAccountProvider) => {
        $crate::__private::PluginKind::StorefrontAccountProvider
    };
    (@kind StorefrontLibraryProvider) => {
        $crate::__private::PluginKind::StorefrontLibraryProvider
    };
    (@export $plugin:ty, StorefrontAccountProvider) => {
        fn link_account(
            plugin: &mut $plugin,
            interaction: &$crate::AccountLinkInteraction,
        ) -> Result<$crate::LinkedAccount, String> {
            <$plugin as $crate::StorefrontAccountProvider>::link_account(plugin, interaction)
        }
    };
    (@export $plugin:ty, StorefrontLibraryProvider) => {
        fn list_games(
            plugin: &mut $plugin,
            account_id: &str,
            credential: Option<&[u8]>,
        ) -> Result<$crate::ListedGames, String> {
            <$plugin as $crate::StorefrontLibraryProvider>::list_games(
                plugin,
                account_id,
                credential,
            )
        }
    };
    ($plugin:ty: [$($capability:ident),+ $(,)?]) => {
        #[doc(hidden)]
        mod __bottles_plugin_export {
            use super::*;

            impl $crate::__private::PluginExports for $plugin {
                fn provides() -> Vec<$crate::__private::PluginKind> {
                    vec![$($crate::export_plugin!(@kind $capability)),+]
                }

                $($crate::export_plugin!(@export $plugin, $capability);)+
            }

            type Component = $crate::__private::Component<$plugin>;

            $crate::__wit::export!(Component with_types_in $crate::__wit);
        }
    };
}

#[doc(hidden)]
pub mod __private {
    use std::{cell::RefCell, marker::PhantomData};

    use crate::{AccountLinkInteraction, LinkedAccount, ListedGames, Plugin};

    pub use super::__wit::exports::bottles::plugin::lifecycle::PluginKind;
    use super::__wit::exports::bottles::plugin::{
        lifecycle::{self, GuestPlugin},
        storefront_account_provider, storefront_library_provider,
    };

    pub struct Component<T>(PhantomData<fn() -> T>);
    pub struct PluginState<T>(RefCell<T>);

    pub trait PluginExports: Plugin {
        fn provides() -> Vec<PluginKind>;

        fn link_account(
            _plugin: &mut Self,
            _interaction: &AccountLinkInteraction,
        ) -> Result<LinkedAccount, String> {
            unreachable!("host invoked an unadvertised storefront account capability")
        }

        fn list_games(
            _plugin: &mut Self,
            _account_id: &str,
            _credential: Option<&[u8]>,
        ) -> Result<ListedGames, String> {
            unreachable!("host invoked an unadvertised storefront library capability")
        }
    }

    impl<T> PluginState<T> {
        fn with_mut<R>(&self, call: impl FnOnce(&mut T) -> Result<R, String>) -> Result<R, String> {
            let mut plugin = self
                .0
                .try_borrow_mut()
                .map_err(|_| "plugin state is already borrowed".to_owned())?;
            call(&mut plugin)
        }
    }

    impl<T: PluginExports> lifecycle::Guest for Component<T> {
        type Plugin = PluginState<T>;

        fn provides() -> Vec<PluginKind> {
            T::provides()
        }
    }

    impl<T: Plugin> GuestPlugin for PluginState<T> {
        fn new() -> Result<lifecycle::Plugin, String> {
            Ok(lifecycle::Plugin::new(Self(RefCell::new(T::new()?))))
        }
    }

    impl<T: PluginExports> storefront_account_provider::Guest for Component<T> {
        fn link_account(
            plugin: lifecycle::PluginBorrow<'_>,
            interaction: &crate::AccountLinkInteraction,
        ) -> Result<crate::LinkedAccount, String> {
            plugin
                .get::<PluginState<T>>()
                .with_mut(|plugin| T::link_account(plugin, interaction))
        }
    }

    impl<T: PluginExports> storefront_library_provider::Guest for Component<T> {
        fn list_games(
            plugin: lifecycle::PluginBorrow<'_>,
            account_id: String,
            credential: Option<Vec<u8>>,
        ) -> Result<crate::ListedGames, String> {
            plugin
                .get::<PluginState<T>>()
                .with_mut(|plugin| T::list_games(plugin, &account_id, credential.as_deref()))
        }
    }
}

/// A small buffered client over the standard WASI HTTP interfaces.
pub mod http_client {
    use std::io::{Read, Write};

    pub use http::{Method, Request, Response};
    use wasip2::http::{
        outgoing_handler,
        types::{
            Fields, IncomingBody, Method as WasiMethod, OutgoingBody, OutgoingRequest, Scheme,
        },
    };

    /// Sends one buffered HTTP request.
    pub fn send(request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, String> {
        let (parts, contents) = request.into_parts();
        let headers = parts
            .headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.as_bytes().to_vec()))
            .collect::<Vec<_>>();
        let headers = Fields::from_list(&headers)
            .map_err(|error| format!("invalid HTTP header: {error:?}"))?;
        let outgoing = OutgoingRequest::new(headers);
        let method = wasi_method(&parts.method);
        outgoing
            .set_method(&method)
            .map_err(|()| "invalid HTTP method".to_owned())?;
        outgoing
            .set_scheme(parts.uri.scheme_str().map(wasi_scheme).as_ref())
            .map_err(|()| "invalid HTTP scheme".to_owned())?;
        outgoing
            .set_authority(parts.uri.authority().map(|authority| authority.as_str()))
            .map_err(|()| "invalid HTTP authority".to_owned())?;
        outgoing
            .set_path_with_query(
                parts
                    .uri
                    .path_and_query()
                    .map(|path_and_query| path_and_query.as_str()),
            )
            .map_err(|()| "invalid HTTP path".to_owned())?;

        let body = outgoing
            .body()
            .map_err(|()| "HTTP request body is unavailable".to_owned())?;
        let future = outgoing_handler::handle(outgoing, None)
            .map_err(|error| format!("HTTP request failed: {error:?}"))?;
        let mut stream = body
            .write()
            .map_err(|()| "HTTP request body stream is unavailable".to_owned())?;
        stream
            .write_all(&contents)
            .map_err(|error| error.to_string())?;
        stream.flush().map_err(|error| error.to_string())?;
        drop(stream);
        OutgoingBody::finish(body, None)
            .map_err(|error| format!("failed to finish HTTP request: {error:?}"))?;

        future.subscribe().block();
        let incoming = future
            .get()
            .ok_or_else(|| "HTTP response was not ready".to_owned())?
            .map_err(|()| "HTTP response was already consumed".to_owned())?
            .map_err(|error| format!("HTTP request failed: {error:?}"))?;

        let mut response = Response::builder().status(incoming.status());
        for (name, value) in incoming.headers().entries() {
            response = response.header(name, value);
        }
        let incoming_body = incoming
            .consume()
            .map_err(|()| "HTTP response body is unavailable".to_owned())?;
        let mut stream = incoming_body
            .stream()
            .map_err(|()| "HTTP response body stream is unavailable".to_owned())?;
        let mut body = Vec::new();
        stream
            .read_to_end(&mut body)
            .map_err(|error| error.to_string())?;
        drop(stream);
        let _trailers = IncomingBody::finish(incoming_body);

        response.body(body).map_err(|error| error.to_string())
    }

    fn wasi_method(method: &Method) -> WasiMethod {
        match method.as_str() {
            "GET" => WasiMethod::Get,
            "HEAD" => WasiMethod::Head,
            "POST" => WasiMethod::Post,
            "PUT" => WasiMethod::Put,
            "DELETE" => WasiMethod::Delete,
            "CONNECT" => WasiMethod::Connect,
            "OPTIONS" => WasiMethod::Options,
            "TRACE" => WasiMethod::Trace,
            "PATCH" => WasiMethod::Patch,
            method => WasiMethod::Other(method.into()),
        }
    }

    fn wasi_scheme(scheme: &str) -> Scheme {
        match scheme {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn maps_standard_and_custom_request_parts() {
            assert!(matches!(wasi_method(&Method::POST), WasiMethod::Post));
            assert!(matches!(
                wasi_method(&Method::from_bytes(b"CUSTOM").unwrap()),
                WasiMethod::Other(method) if method == "CUSTOM"
            ));
            assert!(matches!(wasi_scheme("https"), Scheme::Https));
        }
    }
}

#[doc(hidden)]
pub mod __wit {
    wit_bindgen::generate!({
        path: "wit",
        world: "plugin",
        pub_export_macro: true,
    });
}
