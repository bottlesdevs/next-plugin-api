//! Installed, launchable titles supplied by a plugin.
//! Launch completion means the request finished, not that the title exited.

use crate::Plugin;

#[doc(hidden)]
pub mod __bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "library",
        pub_export_macro: true,
        generate_unused_types: true,
    });
}

pub use __bindings::exports::bottles::plugin::library_provider::LibraryEntry;

/// Installed, launchable titles supplied by a plugin.
#[allow(async_fn_in_trait)]
pub trait LibraryProvider: Plugin {
    async fn list_entries(&self) -> Result<Vec<LibraryEntry>, String>;
    async fn launch(&self, id: String) -> Result<(), String>;
}

#[doc(hidden)]
#[macro_export]
macro_rules! __bottles_library_glue {
    ($plugin:ident, $state:ident) => {
        impl $crate::library::__bindings::exports::bottles::plugin::library_provider::Guest for $plugin {
            async fn list_entries() -> Result<Vec<$crate::library::LibraryEntry>, String> {
                <$plugin as $crate::library::LibraryProvider>::list_entries(
                    $state.get_or_init(<$plugin as $crate::Plugin>::new),
                ).await
            }

            async fn launch(id: String) -> Result<(), String> {
                <$plugin as $crate::library::LibraryProvider>::launch(
                    $state.get_or_init(<$plugin as $crate::Plugin>::new), id,
                ).await
            }
        }
        $crate::library::__bindings::export!($plugin with_types_in $crate::library::__bindings);
    };
}

#[doc(hidden)]
pub use crate::__bottles_library_glue as __glue;
