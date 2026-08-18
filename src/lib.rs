//! Rust API for Bottles plugins.

/// The exact WIT/API version implemented by this crate.
pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use wit::AccountIdentity;

/// A Bottles plugin.
pub trait Plugin {
    /// Creates the persistent plugin instance.
    fn new() -> Self
    where
        Self: Sized;

    /// Links an account to the profile identified by `profile_id`.
    fn link_account(&mut self, _profile_id: String) -> Result<AccountIdentity, String> {
        Err("storefront account integration is not implemented".into())
    }
}

/// Registers a type as the plugin exported by the current WebAssembly component.
#[macro_export]
macro_rules! register_plugin {
    ($plugin:ty) => {
        #[unsafe(export_name = "init-plugin")]
        pub extern "C" fn __bottles_init_plugin() {
            $crate::__private::register_plugin(|| Box::new(<$plugin as $crate::Plugin>::new()));
        }
    };
}

#[doc(hidden)]
pub mod __private {
    use super::{AccountIdentity, Plugin};

    pub fn register_plugin(build: fn() -> Box<dyn Plugin>) {
        // SAFETY: the component host serializes initialization and calls for each instance.
        unsafe { super::PLUGIN = Some(build()) }
    }

    pub fn link_account(profile_id: String) -> Result<AccountIdentity, String> {
        // SAFETY: each Wasm instance has its own static and the host serializes calls.
        let plugin = unsafe { &mut *(&raw mut super::PLUGIN) }
            .as_mut()
            .expect("init-plugin must run before link-account");
        plugin.link_account(profile_id)
    }
}

static mut PLUGIN: Option<Box<dyn Plugin>> = None;

#[cfg(target_arch = "wasm32")]
#[unsafe(link_section = "bottles:api-version")]
#[doc(hidden)]
pub static BOTTLES_API_VERSION: [u8; 6] = *include_bytes!(concat!(env!("OUT_DIR"), "/api-version"));

mod wit {
    wit_bindgen::generate!({
        path: "wit",
        world: "plugin",
        skip: ["init-plugin"],
    });
}

wit::export!(_PluginComponent with_types_in wit);

struct _PluginComponent;

impl wit::Guest for _PluginComponent {
    fn link_account(profile_id: String) -> Result<AccountIdentity, String> {
        __private::link_account(profile_id)
    }
}
