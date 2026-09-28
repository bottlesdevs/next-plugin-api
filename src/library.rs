//! Installed, launchable titles supplied by a plugin.
//! Launch completion means the request finished, not that the title exited.

use crate::Plugin;

#[doc(hidden)]
pub mod __bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "library",
        disable_custom_section_link_helpers: true,
        generate_unused_types: true,
    });
}

pub use __bindings::exports::bottles::plugin::library_provider::LibraryEntry;

/// Installed, launchable titles supplied by a plugin.
#[allow(async_fn_in_trait)]
#[bottles_plugin_macros::provider(library)]
pub trait LibraryProvider: Plugin {
    async fn list_entries(&self) -> Result<Vec<LibraryEntry>, String>;
    async fn launch(&self, id: String) -> Result<(), String>;
}
