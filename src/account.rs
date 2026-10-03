//! Account linking through an explicit host interaction capability.

use crate::Plugin;

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

pub use __bindings::exports::bottles::plugin::account_provider::{AccountIdentity, LinkedAccount};

/// Account linking through an explicit host interaction capability.
#[allow(async_fn_in_trait)]
pub trait AccountProvider: Plugin {
    async fn link_account(&self, interaction: &Interaction) -> Result<LinkedAccount, String>;
}

#[doc(hidden)]
#[macro_export]
macro_rules! __bottles_account_glue {
    ($plugin:ident, $state:ident) => {
        impl $crate::account::__bindings::exports::bottles::plugin::account_provider::Guest for $plugin {
            async fn link_account(
                interaction: &$crate::account::Interaction,
            ) -> Result<$crate::account::LinkedAccount, String> {
                <$plugin as $crate::account::AccountProvider>::link_account(
                    $state.get_or_init(<$plugin as $crate::Plugin>::new),
                    interaction,
                ).await
            }
        }
        $crate::account::__bindings::export!($plugin with_types_in $crate::account::__bindings);
    };
}

#[doc(hidden)]
pub use crate::__bottles_account_glue as __glue;
