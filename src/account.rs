//! Links a user account through caller-supplied interaction.
//!
//! Implement [`AccountProvider`] and select `account` in [`crate::export!`]. The
//! host supplies an [`Interaction`] for the current call; the provider can ask
//! the user to visit a URL and return input needed by its authentication flow.
//! Return public identity separately from private credential bytes.
//!
//! Provider state can retain an authenticated session for other capabilities in
//! the same guest instance. That state is lost when the instance is unloaded;
//! the SDK has no import for retrieving stored credentials on a later load.
//!
//! # Examples
//!
//! ```text
//! use bottles_plugin_api::account::{AccountIdentity, LinkedAccount};
//! let linked = LinkedAccount {
//!     identity: AccountIdentity {
//!         account_id: "provider-account-42".into(),
//!         display_name: "Player".into(),
//!     },
//!     credential: None,
//! };
//! ```

#[doc(hidden)]
pub mod __bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "account",
        pub_export_macro: true,
        generate_unused_types: true,
    });
}

/// Requests user input from the caller during account linking.
///
/// Its async `request_input(url, instructions)` method returns the caller's input
/// or an error string. The URL must be an absolute URL accepted by the host. The
/// caller decides how to present it and obtain input; this interface does not
/// itself authenticate the account.
///
/// The resource is borrowed for one [`AccountProvider::link_account`] call. Do
/// not retain it in shared plugin state after that call.
///
/// # Examples
///
/// ```text
/// let code = interaction.request_input(
///     "https://example.com/login".into(),
///     "Sign in, then paste the authorization code.".into(),
/// ).await?;
/// ```
pub use __bindings::bottles::plugin::account_link::Interaction;

/// Identifies an account within this provider.
///
/// `account_id` is the provider's stable account identifier; `display_name` is
/// the human-readable name shown to the user. Neither field should contain a
/// private credential.
///
/// # Examples
///
/// ```text
/// use bottles_plugin_api::account::AccountIdentity;
/// let identity = AccountIdentity {
///     account_id: "provider-account-42".into(),
///     display_name: "Player".into(),
/// };
/// ```
pub use __bindings::exports::bottles::plugin::account_provider::AccountIdentity;
use __bindings::exports::bottles::plugin::account_provider::Guest;
/// Returns public account identity and optional private credential bytes.
///
/// `identity` is suitable for profile metadata. `credential` is an opaque byte
/// vector whose format the provider chooses. When the caller links through
/// Bottles core, it stores these bytes separately in the platform credential
/// store. Returning `None` means there is no credential to store.
///
/// # Examples
///
/// ```text
/// use bottles_plugin_api::account::LinkedAccount;
/// let linked = LinkedAccount { identity, credential: None };
/// ```
pub use __bindings::exports::bottles::plugin::account_provider::LinkedAccount;

/// Identifies or authenticates an account for linking to a profile.
///
/// Implement this trait on the shared state type and select `account` in
/// [`crate::export!`]. Provider calls may overlap with each other and with calls
/// to other capabilities; release shared-state locks before awaiting input or
/// network responses.
///
/// # Examples
///
/// ```text
/// impl bottles_plugin_api::account::AccountProvider for MyPlugin {
///     async fn link_account(
///         &self,
///         interaction: &bottles_plugin_api::account::Interaction,
///     ) -> Result<bottles_plugin_api::account::LinkedAccount, String> {
///         let code = interaction.request_input(
///             "https://example.com/login".into(),
///             "Paste the authorization code after signing in.".into(),
///         ).await?;
///         self.exchange_code_and_link(code).await
///     }
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait AccountProvider {
    /// Runs the account-link flow using the supplied interaction resource.
    ///
    /// Return an account identity only after the provider's flow succeeds.
    /// Include credential bytes only when there is a secret for the caller to
    /// store. The interaction belongs to this invocation, not to the returned
    /// account or the shared plugin state.
    ///
    /// # Errors
    ///
    /// Returns a provider-defined error string when authentication or account
    /// identification fails. Errors from [`Interaction::request_input`] and
    /// network calls can be propagated as part of that string.
    ///
    /// # Examples
    ///
    /// ```text
    /// let linked = provider.link_account(interaction).await?;
    /// let account_id = linked.identity.account_id;
    /// ```
    async fn link_account(&self, interaction: &Interaction) -> Result<LinkedAccount, String>;
}

impl<P: AccountProvider + crate::Plugin> Guest for P {
    async fn link_account(interaction: &Interaction) -> Result<LinkedAccount, String> {
        P::instance().link_account(interaction).await
    }
}
