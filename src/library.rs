//! Lists installed titles and requests launches through a provider.
//!
//! Implement [`LibraryProvider`] on the guest state type and select `library`
//! in [`crate::export!`]. Entry identifiers belong to the provider, so the same
//! title must use the same identifier across list calls. The host associates
//! each entry with its provider when exposing it to the caller.
//!
//! A successful launch means the request finished; it does not mean the title
//! exited. The SDK does not supply a host process-execution import.
//!
//! # Examples
//!
//! ```text
//! use bottles_plugin_api::library::LibraryEntry;
//! let entry = LibraryEntry { id: "installed-game-42".into(), title: "Example Game".into() };
//! ```

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
/// Names one installed title the provider can launch.
///
/// `id` is an opaque identifier local to the provider, passed back unchanged to
/// [`LibraryProvider::launch`]. `title` is the human-readable display name.
///
/// # Examples
///
/// ```text
/// use bottles_plugin_api::library::LibraryEntry;
/// let entry = LibraryEntry {
///     id: "installed-game-42".into(),
///     title: "Example Game".into(),
/// };
/// ```
pub use __bindings::exports::bottles::plugin::library_provider::LibraryEntry;

/// Supplies installed titles and handles launch requests for their identifiers.
///
/// Implement this trait on a type with [`Default`] and select `library` in
/// [`crate::export!`]. All selected providers receive the same state instance;
/// account linking can, for example, populate a session used by listing.
///
/// # Examples
///
/// ```text
/// use bottles_plugin_api::library::{LibraryEntry, LibraryProvider};
///
/// #[derive(Default)]
/// struct Catalog;
///
/// impl LibraryProvider for Catalog {
///     async fn list_entries(&self) -> Result<Vec<LibraryEntry>, String> {
///         Ok(Vec::new())
///     }
///
///     async fn launch(&self, id: String) -> Result<(), String> {
///         Err(format!("No installed title with ID {id}"))
///     }
/// }
///
/// bottles_plugin_api::export!(Catalog: library);
/// ```
#[allow(async_fn_in_trait)]
pub trait LibraryProvider {
    /// Returns the provider's current installed, launchable titles.
    ///
    /// An empty vector represents a successful query with no titles to list.
    /// Keep entry identifiers stable so later launches can locate the title.
    ///
    /// # Errors
    ///
    /// Returns a provider-defined error string if the catalog cannot be queried,
    /// for example because its authenticated session is unavailable or a remote
    /// service request fails.
    ///
    /// # Examples
    ///
    /// ```text
    /// let entries = provider.list_entries().await?;
    /// ```
    async fn list_entries(&self) -> Result<Vec<LibraryEntry>, String>;
    /// Requests a launch for an identifier returned by [`Self::list_entries`].
    ///
    /// Completion means the launch request finished, not that the title exited.
    ///
    /// # Errors
    ///
    /// Returns a provider-defined error string if the title cannot be found or
    /// its launch request fails.
    ///
    /// # Examples
    ///
    /// ```text
    /// provider.launch(entry.id).await?;
    /// ```
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
