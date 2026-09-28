//! Account linking through an explicit host interaction capability.

use crate::Plugin;

#[doc(hidden)]
pub mod __bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "account",
        disable_custom_section_link_helpers: true,
        generate_unused_types: true,
    });
}

/// A host interaction used during account linking.
pub use __bindings::bottles::plugin::account_link::Interaction;

pub use __bindings::exports::bottles::plugin::account_provider::{AccountIdentity, LinkedAccount};

/// Account linking through an explicit host interaction capability.
#[allow(async_fn_in_trait)]
#[bottles_plugin_macros::provider(account)]
pub trait AccountProvider: Plugin {
    async fn link_account(&self, interaction: &Interaction) -> Result<LinkedAccount, String>;
}
