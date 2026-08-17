//! Rust API for Bottles plugins.
//!
//! Plugins are persistent WebAssembly components. Host operations performed through a
//! [`Bottle`] take effect immediately and are not rolled back if the plugin later fails.

/// The exact WIT/API version implemented by this crate.
pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use wit::bottles::plugin::bottle::Bottle;
pub use wit::bottles::plugin::filesystem::ReadRoot;
pub use wit::bottles::plugin::types::{
    Architecture, Command, Component, ComponentSlot, Dependency, DllOverride, DllOverrideMode,
    EnvironmentVariable, LogLevel, NewProgram, OperatingSystem, Platform, Program, Storage,
};
pub use wit::bottles::plugin::{
    filesystem, http_client, key_value_store, logging, platform, process, settings,
};

/// A Bottles plugin.
pub trait Plugin {
    /// Creates the persistent plugin instance.
    fn new() -> Self
    where
        Self: Sized;

    /// Runs one command declared by the plugin manifest.
    fn run_command(
        &mut self,
        command: Command,
        arguments: Vec<String>,
        bottle: Option<&Bottle>,
    ) -> Result<String, String>;
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
    use super::Plugin;

    pub fn register_plugin(build: fn() -> Box<dyn Plugin>) {
        // SAFETY: the component host serializes initialization and calls for each instance.
        unsafe { super::PLUGIN = Some(build()) }
    }
}

fn plugin() -> &'static mut dyn Plugin {
    #[expect(static_mut_refs)]
    // SAFETY: the component host initializes first and serializes all calls to this instance.
    unsafe {
        PLUGIN
            .as_deref_mut()
            .expect("Bottles plugin invoked before init-plugin")
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

wit::export!(PluginComponent with_types_in wit);

struct PluginComponent;

impl wit::Guest for PluginComponent {
    fn run_command(
        command: Command,
        arguments: Vec<String>,
        bottle: Option<&Bottle>,
    ) -> Result<String, String> {
        plugin().run_command(command, arguments, bottle)
    }
}
