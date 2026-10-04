# Bottles plugin guest SDK

`bottles-plugin-api` supplies the Rust provider traits and WebAssembly component
exports for Bottles plugins. Use it in a guest crate built as a `cdylib` for
`wasm32-wasip3`. Native applications load those components through
`bottles-plugin-host`.

## Getting started

Implement `Default` and a provider trait, then select the capability in `export!`.
This library provider has no installed titles yet:

```text
use bottles_plugin_api::library::{LibraryEntry, LibraryProvider};

#[derive(Default)]
struct Catalog;

impl LibraryProvider for Catalog {
    async fn list_entries(&self) -> Result<Vec<LibraryEntry>, String> {
        Ok(Vec::new())
    }

    async fn launch(&self, id: String) -> Result<(), String> {
        Err(format!("No installed title with ID {id}"))
    }
}

bottles_plugin_api::export!(Catalog: library);
```

Set the guest's library target in its `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]
```

Build with the SDK's supported guest toolchain:

```sh
cargo +nightly-2026-09-25 build --release --target wasm32-wasip3
```

A dependency's `rust-toolchain.toml` does not select the consuming crate's
toolchain. Select it explicitly with the command above or the guest crate's own
toolchain file. The build produces a component in
`target/wasm32-wasip3/release/<crate_name>.wasm`.

## State and capability selection

[`export!`] implements [`Plugin`] for the selected type. On first use, it calls
[`Default::default()`] and stores the result in one [`std::sync::OnceLock`].
Every exported capability uses that same instance for the lifetime of the loaded
component. Loading the component again creates a separate guest instance;
in-memory state is not persistent storage.

Calls may run concurrently. Use interior mutability, such as a
[`std::sync::Mutex`], for state changed through `&self`. Release mutex guards
before `.await` so another call can access the state while a request is pending.

The export list chooses the component's capabilities:

```text
bottles_plugin_api::export!(MyPlugin: account, library);
```

Implementing a provider trait does not export its interface. For example, a type
may implement both provider traits while `export!(MyPlugin: library)` exports
only the library interface. Call `export!` once for the type, listing all selected
capabilities together.

## Account linking and HTTP

[`account::AccountProvider`] receives a borrowed [`account::Interaction`] for
requesting user input during that invocation. It returns public account identity
and optional opaque credential bytes. When linking through Bottles core, identity
is stored with the profile and credentials are stored separately in the platform
credential store. The SDK currently has no credential retrieval interface; a
later guest load does not restore an in-memory login session automatically.

[`http_client::send`] uses standard WASI HTTP imports. Both the request and
response bodies are fully buffered in memory; it does not provide a streaming
body API.

## Packaging

Place the compiled component at `plugin.wasm` beside a `plugin.toml` manifest:

```toml
id = "my-plugin"
name = "My Plugin"
version = "0.1.0"
```

The host reads these filenames when installing or loading a package. Package
version is display metadata; exported interface versions come from the SDK's WIT
definitions.
