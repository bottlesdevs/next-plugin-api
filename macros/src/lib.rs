use std::collections::{BTreeSet, HashSet};

use heck::{ToKebabCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    FnArg, Ident, ItemTrait, LitStr, Pat, Token, TraitItem, parse::Parse, parse_macro_input,
    punctuated::Punctuated,
};
use wit_parser::{
    FunctionKind, Handle, Param, Resolve, Type, TypeDefKind, TypeId, TypeOwner, WorldItem,
};

struct Export {
    plugin: Ident,
    providers: Punctuated<Ident, Token![,]>,
}

impl Parse for Export {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let plugin = input.parse()?;
        input.parse::<Token![,]>()?;
        let providers = Punctuated::parse_terminated(input)?;
        if providers.is_empty() {
            return Err(input.error("select at least one provider"));
        }
        Ok(Self { plugin, providers })
    }
}

#[proc_macro_attribute]
/// Generates the guest adapter for a trait that forwards a WIT world's async
/// functions from `self: borrow<plugin>` to `&self`.
pub fn provider(attr: TokenStream, item: TokenStream) -> TokenStream {
    let world = parse_macro_input!(attr as Ident);
    let trait_item = parse_macro_input!(item as ItemTrait);
    match provider_impl(world, trait_item) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn provider_impl(world: Ident, trait_item: ItemTrait) -> syn::Result<TokenStream2> {
    if !trait_item.generics.params.is_empty() || trait_item.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &trait_item.ident,
            "provider traits cannot be generic",
        ));
    }

    let mut resolve = wit_parser::Resolve::default();
    let package = resolve
        .push_str("plugin.wit", include_str!("../../wit/plugin.wit"))
        .expect("plugin WIT must be valid");
    let world_name = world.to_string().to_kebab_case();
    let world_id = resolve.packages[package]
        .worlds
        .get(&world_name)
        .ok_or_else(|| syn::Error::new_spanned(&world, "unknown WIT provider world"))?;
    let state_id = resolve.packages[package]
        .interfaces
        .get("plugin-state")
        .ok_or_else(|| syn::Error::new_spanned(&world, "plugin-state interface is missing"))?;
    let plugin_resource = *resolve.interfaces[*state_id]
        .types
        .get("plugin")
        .expect("plugin-state must define a plugin resource");
    let exports: Vec<_> = resolve.worlds[*world_id]
        .exports
        .values()
        .filter_map(|item| match item {
            WorldItem::Interface { id, .. } if id != state_id => Some(*id),
            _ => None,
        })
        .collect();
    if exports.len() != 1 {
        return Err(syn::Error::new_spanned(
            &world,
            "provider helper requires one exported interface besides plugin-state",
        ));
    }
    let interface = &resolve.interfaces[exports[0]];
    let interface_name = interface
        .name
        .as_ref()
        .expect("provider interface must be named");
    let interface_package =
        &resolve.packages[interface.package.expect("interface must have a package")].name;
    let namespace = format_ident!("{}", interface_package.namespace.to_snake_case());
    let package_name = format_ident!("{}", interface_package.name.to_snake_case());
    let interface_module = format_ident!("{}", interface_name.to_snake_case());
    let state_interface = &resolve.interfaces[*state_id];
    let state_package = &resolve.packages[state_interface
        .package
        .expect("interface must have a package")]
    .name;
    let state_namespace = format_ident!("{}", state_package.namespace.to_snake_case());
    let state_package_name = format_ident!("{}", state_package.name.to_snake_case());
    let state_module = format_ident!("plugin_state");

    let mut methods = Vec::new();
    let mut remaining: BTreeSet<_> = interface.functions.keys().cloned().collect();
    for item in &trait_item.items {
        let TraitItem::Fn(method) = item else {
            return Err(syn::Error::new_spanned(
                item,
                "provider traits support methods only",
            ));
        };
        let signature = &method.sig;
        if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
            return Err(syn::Error::new_spanned(
                signature,
                "provider methods cannot be generic",
            ));
        }
        let wit_name = signature.ident.to_string().to_kebab_case();
        let function = interface.functions.get(&wit_name).ok_or_else(|| {
            syn::Error::new_spanned(&signature.ident, "method is not in the WIT interface")
        })?;
        remaining.remove(&wit_name);
        if !matches!(function.kind, FunctionKind::AsyncFreestanding)
            || signature.asyncness.is_none()
            || function
                .params
                .first()
                .is_none_or(|param| !borrows_resource(&resolve, param, plugin_resource))
            || function.params.len() != signature.inputs.len()
        {
            return Err(syn::Error::new_spanned(
                signature,
                "expected an async WIT function starting with borrow<plugin> and matching arguments",
            ));
        }
        let Some(FnArg::Receiver(receiver)) = signature.inputs.first() else {
            return Err(syn::Error::new_spanned(signature, "expected &self"));
        };
        if receiver.reference.is_none()
            || receiver.mutability.is_some()
            || receiver.colon_token.is_some()
        {
            return Err(syn::Error::new_spanned(receiver, "expected &self"));
        }
        let mut inputs = Vec::new();
        let mut arguments = Vec::new();
        for input in signature.inputs.iter().skip(1) {
            let FnArg::Typed(input) = input else {
                return Err(syn::Error::new_spanned(input, "expected a named argument"));
            };
            let Pat::Ident(argument) = input.pat.as_ref() else {
                return Err(syn::Error::new_spanned(
                    &input.pat,
                    "expected a named argument",
                ));
            };
            inputs.push(input);
            arguments.push(&argument.ident);
        }
        let name = &signature.ident;
        let output = &signature.output;
        let trait_name = &trait_item.ident;
        methods.push(quote! {
            async fn #name(
                state: $bindings::exports::#state_namespace::#state_package_name::#state_module::PluginBorrow<'_>,
                #(#inputs),*
            ) #output {
                <$plugin as $crate::#world::#trait_name>::#name(
                    state.get::<$plugin>(),
                    #(#arguments),*
                ).await
            }
        });
    }
    if !remaining.is_empty() {
        return Err(syn::Error::new_spanned(
            &trait_item.ident,
            format!(
                "missing WIT methods: {}",
                remaining.into_iter().collect::<Vec<_>>().join(", ")
            ),
        ));
    }

    let glue_name = format_ident!("__bottles_{}_glue", world.to_string().to_snake_case());
    Ok(quote! {
        #trait_item

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #glue_name {
            ($plugin:ident, $bindings:ident) => {
                const _: () = {
                    #[allow(unused_imports)]
                    use $crate::#world::*;
                    impl $bindings::exports::#namespace::#package_name::#interface_module::Guest for $plugin {
                        #(#methods)*
                    }
                };
            };
        }
    })
}

fn borrows_resource(resolve: &Resolve, param: &Param, resource: TypeId) -> bool {
    let Type::Id(id) = param.ty else {
        return false;
    };
    let TypeDefKind::Handle(Handle::Borrow(borrowed)) =
        &resolve.types[underlying_type(resolve, id)].kind
    else {
        return false;
    };
    underlying_type(resolve, *borrowed) == underlying_type(resolve, resource)
}

fn underlying_type(resolve: &Resolve, mut id: TypeId) -> TypeId {
    while let TypeDefKind::Type(Type::Id(next)) = &resolve.types[id].kind {
        id = *next;
    }
    id
}

#[proc_macro]
/// Exports one guest object with the selected WIT provider worlds.
/// Each world uses an SDK adapter macro generated by [`provider`] or written by hand.
pub fn export(input: TokenStream) -> TokenStream {
    let Export { plugin, providers } = parse_macro_input!(input as Export);
    let mut resolve = wit_parser::Resolve::default();
    let mut wit = include_str!("../../wit/plugin.wit").to_owned();
    let package = resolve
        .push_str("plugin.wit", &wit)
        .expect("plugin WIT must be valid");
    let plugin_state_id = resolve.packages[package].name.interface_id("plugin-state");

    let mut names = HashSet::new();
    let mut selected = Vec::new();
    for provider in providers {
        let world_name = provider.to_string().to_kebab_case();
        if !names.insert(world_name.clone()) {
            return syn::Error::new_spanned(provider, "duplicate provider")
                .to_compile_error()
                .into();
        }
        let Some(&world) = resolve.packages[package].worlds.get(&world_name) else {
            return syn::Error::new_spanned(provider, "unknown WIT provider world")
                .to_compile_error()
                .into();
        };
        selected.push((provider, world_name, world));
    }

    wit.push_str("\nworld selected-plugin {\n");
    for (_, world, _) in &selected {
        wit.push_str("    include ");
        wit.push_str(world);
        wit.push_str(";\n");
    }
    wit.push_str("}\n");
    let wit = LitStr::new(&wit, Span::call_site());

    let mut with = Vec::<TokenStream2>::new();
    let mut mapped = HashSet::new();
    let mut has_plugin_state = false;
    for (provider, _, world) in &selected {
        let world = &resolve.worlds[*world];
        for item in world.imports.values() {
            let WorldItem::Interface { id, .. } = item else {
                continue;
            };
            let interface = &resolve.interfaces[*id];
            let Some(name) = &interface.name else {
                continue;
            };
            let key = resolve
                .id_of(*id)
                .expect("named WIT interface must have an ID");
            if !mapped.insert(key.clone()) {
                continue;
            }
            let package =
                &resolve.packages[interface.package.expect("interface must have a package")].name;
            let namespace = format_ident!("{}", package.namespace.to_snake_case());
            let package_name = format_ident!("{}", package.name.to_snake_case());
            let module = format_ident!("{}", name.to_snake_case());
            let key = LitStr::new(&key, Span::call_site());
            with.push(quote! {
                #key: ::bottles_plugin_api::#provider::__bindings::#namespace::#package_name::#module,
            });
        }

        for item in world.exports.values() {
            let WorldItem::Interface {
                id: interface_id, ..
            } = item
            else {
                continue;
            };
            let interface = &resolve.interfaces[*interface_id];
            let Some(name) = &interface.name else {
                continue;
            };
            let id = resolve
                .id_of(*interface_id)
                .expect("named WIT interface must have an ID");
            has_plugin_state |= id == plugin_state_id;
            let package =
                &resolve.packages[interface.package.expect("interface must have a package")].name;
            let namespace = format_ident!("{}", package.namespace.to_snake_case());
            let package_name = format_ident!("{}", package.name.to_snake_case());
            let module = format_ident!("{}", name.to_snake_case());
            for (type_name, type_id) in &interface.types {
                let definition = &resolve.types[*type_id];
                if definition.owner != TypeOwner::Interface(*interface_id) {
                    continue;
                }
                if !matches!(
                    definition.kind,
                    TypeDefKind::Record(_)
                        | TypeDefKind::Variant(_)
                        | TypeDefKind::Enum(_)
                        | TypeDefKind::Flags(_)
                ) {
                    continue;
                }
                let key = format!("{id}/{type_name}");
                if !mapped.insert(key.clone()) {
                    continue;
                }
                let key = LitStr::new(&key, Span::call_site());
                let ty = format_ident!("{}", type_name.to_upper_camel_case());
                with.push(quote! {
                    #key: ::bottles_plugin_api::#provider::__bindings::exports::#namespace::#package_name::#module::#ty,
                });
            }
        }
    }

    let state_glue = has_plugin_state.then(|| quote! {
        impl __bottles_bindings::exports::bottles::plugin::plugin_state::Guest for #plugin {
            type Plugin = #plugin;
        }

        impl __bottles_bindings::exports::bottles::plugin::plugin_state::GuestPlugin for #plugin {
            fn new() -> Self {
                <Self as ::bottles_plugin_api::Plugin>::new()
            }
        }
    });
    let provider_glue = selected.iter().map(|(provider, _, _)| {
        let glue_name = format_ident!("__bottles_{}_glue", provider.to_string().to_snake_case());
        quote! {
            ::bottles_plugin_api::#glue_name!(#plugin, __bottles_bindings);
        }
    });

    quote! {
        #[doc(hidden)]
        mod __bottles_bindings {
            ::bottles_plugin_api::__private::wit_bindgen::generate!({
                path: [],
                inline: #wit,
                world: "selected-plugin",
                runtime_path: "::bottles_plugin_api::__private::wit_bindgen::rt",
                generate_unused_types: true,
                with: {
                    #(#with)*
                },
            });
        }

        #state_glue
        #(#provider_glue)*

        __bottles_bindings::export!(#plugin with_types_in __bottles_bindings);
    }
    .into()
}
