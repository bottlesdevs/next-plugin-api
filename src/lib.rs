//! Guest bindings for Bottles plugin interfaces.

/// Account linking through an explicit host interaction capability.
pub mod account {
    wit_bindgen::generate!({ path: "wit", world: "account", pub_export_macro: true });
    pub use bottles::plugin::account_link::Interaction;
    pub use exports::bottles::plugin::account_provider::*;
}

/// Optional authenticated library enumeration.
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
