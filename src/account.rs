//! Account linking through an explicit host interaction capability.

#[doc(hidden)]
pub mod __bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "account",
        pub_export_macro: true,
        generate_unused_types: true,
    });
}

/// A host interaction used during account linking.
pub use __bindings::bottles::plugin::account_link::Interaction;

use __bindings::exports::bottles::plugin::account_provider::Guest;
pub use __bindings::exports::bottles::plugin::account_provider::{AccountIdentity, LinkedAccount};

/// Account linking through an explicit host interaction capability.
#[allow(async_fn_in_trait)]
pub trait AccountProvider {
    async fn link_account(&self, interaction: &Interaction) -> Result<LinkedAccount, String>;
}

impl<P: AccountProvider + crate::Plugin> Guest for P {
    async fn link_account(interaction: &Interaction) -> Result<LinkedAccount, String> {
        P::instance().link_account(interaction).await
    }
}
