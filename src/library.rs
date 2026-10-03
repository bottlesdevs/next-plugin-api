//! Installed, launchable titles supplied by a plugin.
//! Launch completion means the request finished, not that the title exited.

#[doc(hidden)]
pub mod __bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "library",
        pub_export_macro: true,
        generate_unused_types: true,
    });
}

use __bindings::exports::bottles::plugin::library_provider::Guest;
pub use __bindings::exports::bottles::plugin::library_provider::LibraryEntry;

/// Installed, launchable titles supplied by a plugin.
#[allow(async_fn_in_trait)]
pub trait LibraryProvider {
    async fn list_entries(&self) -> Result<Vec<LibraryEntry>, String>;
    async fn launch(&self, id: String) -> Result<(), String>;
}

impl<P: LibraryProvider + crate::Plugin> Guest for P {
    async fn list_entries() -> Result<Vec<LibraryEntry>, String> {
        P::instance().list_entries().await
    }

    async fn launch(id: String) -> Result<(), String> {
        P::instance().launch(id).await
    }
}
