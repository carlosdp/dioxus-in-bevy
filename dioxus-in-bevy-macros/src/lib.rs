use proc_macro::TokenStream;
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::{
    braced, parse::Parse, parse_macro_input, punctuated::Punctuated, ExprPath, Ident, Token,
};

struct ElementModules {
    elements: Punctuated<ExprPath, Token![,]>,
    events: Option<Punctuated<ExprPath, Token![,]>>,
}

impl Parse for ElementModules {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse elements: { mod1, mod2, ... }
        input.parse::<syn::Ident>()?; // Parse "elements" keyword
        input.parse::<Token![:]>()?;
        let content;
        braced!(content in input);
        let elements = Punctuated::<ExprPath, Token![,]>::parse_terminated(&content)?;

        // Optionally parse events
        let events = if input.peek(syn::Ident) && input.peek2(Token![:]) {
            let _ = input.parse::<syn::Ident>()?; // Parse "events" keyword
            input.parse::<Token![:]>()?;
            let content;
            braced!(content in input);
            let events = Punctuated::<ExprPath, Token![,]>::parse_terminated(&content)?;
            Some(events)
        } else {
            None
        };

        Ok(ElementModules { elements, events })
    }
}

#[proc_macro]
pub fn dioxus_elements(input: TokenStream) -> TokenStream {
    let ElementModules {
        elements: element_tokens,
        events,
    } = parse_macro_input!(input as ElementModules);

    // Generate code to expand each module's all_elements! macro
    let element_imports = element_tokens.iter().map(|module| {
        quote! { pub use #module::*; }
    });

    let element_paths = element_tokens.iter().map(|module| {
        quote! { #module }
    });

    // Handle event imports if present
    let event_imports = if let Some(events) = events {
        events
            .iter()
            .map(|module| {
                quote! { pub use super::#module::*; }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    // Generate the final code
    let output = quote! {
        pub mod dioxus_elements {
            use super::*;
            pub use elements::*;

            pub mod elements {
                use super::*;

                #(#element_imports)*

                #[allow(unused_macros)]
                macro_rules! build_completions {
                    ($($element:ident),*) => {
                        #[doc(hidden)]
                        #[allow(unused)]
                        pub mod completions {
                            #[allow(non_camel_case_types)]
                            pub enum CompleteWithBraces {
                                $($element {}),*
                            }
                        }
                    }
                }

                macro_rules! expand_all_elements {
                    // Base case: no more modules to process
                    (@process | $callback:ident | $($accum:ident),* ; ) => {
                        $callback! { $($accum),* }
                    };

                    // Recursive case: process one module path, then continue
                    (@process | $callback:ident | $($accum:ident),* ; $last:path) => {
                        dioxus_in_bevy::paste::paste! {
                            [<$last>]::macros::[<all_ $last _elements>]! {
                                @collect_and_continue |
                                expand_all_elements $callback |
                                $($accum),* ;
                            }
                        }
                    };

                    (@process | $callback:ident | $($accum:ident),* ; $next:path, $($rest:path),*) => {
                        dioxus_in_bevy::paste::paste! {
                            [<$next>]::macros::[<all_ $next _elements>]! {
                                @collect_and_continue |
                                expand_all_elements $callback |
                                $($accum),* ;
                                $($rest),*
                            }
                        }
                    };

                    // Entry point
                    ($callback:ident | $($module:path),+) => {
                        expand_all_elements! { @process | $callback | ; $($module),* }
                    };
                }

                expand_all_elements! { build_completions | #(#element_paths),* }
            }

            pub mod events {
                #(#event_imports)*
            }
        }
    };

    output.into()
}

struct ElementList {
    module_name: Ident,
    elements: Punctuated<Ident, Token![,]>,
}

impl Parse for ElementList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let module_name = input.parse::<syn::Ident>()?;
        let elements = Punctuated::<Ident, Token![,]>::parse_terminated(&input)?;

        Ok(ElementList {
            module_name,
            elements,
        })
    }
}

#[proc_macro]
pub fn create_all_elements(input: TokenStream) -> TokenStream {
    let ElementList {
        module_name,
        elements,
    } = parse_macro_input!(input as ElementList);

    let final_elements = elements.iter().map(|element| {
        quote! {
            #element
        }
    });

    let inner_elements = elements.iter().map(|element| {
        quote! {
            #element
        }
    });

    let macro_ident = format_ident!("all_{}_elements", module_name);

    let output = quote! {
        pub mod macros {
            #[macro_export]
            macro_rules! #macro_ident {
                // Original behavior
                () => {
                    #(#final_elements),*
                 }; // Your actual elements here

                // New collector pattern for use with expand_all_elements
                (@collect_and_continue | $macro_name:ident $callback:ident | $($accum:ident),* ; $($rest:path),*) => {
                    $macro_name!(@process | $callback | $($accum,)* #(#inner_elements),* ; $($rest),*);
                };
            }

            pub use #macro_ident;
        }
    };

    output.into()
}

#[proc_macro_attribute]
pub fn bevy_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the incoming item as a function. We assume the body is valid Rust (e.g. `rsx!{ ... }`).
    // If parsing fails, just return the original tokens so the compiler can flag the error.
    let input_fn: syn::ItemFn = match syn::parse(item.clone()) {
        Ok(func) => func,
        Err(_) => {
            return item;
        }
    };

    // Keep all attributes except our own and add the #[component] attribute from Dioxus.
    let mut attrs = input_fn.attrs.clone();
    attrs.retain(|a| !a.path().is_ident("bevy_component"));
    attrs.push(syn::parse_quote!(#[component]));

    let vis = &input_fn.vis;
    let sig = &input_fn.sig;

    // Attempt to extract the inner tokens of an `rsx! {...}` invocation so we can avoid double‐wrapping.
    let user_tokens: proc_macro2::TokenStream = {
        // Helper to convert the entire original statements into a token stream (fallback behaviour)
        let build_fallback = || {
            let mut ts = proc_macro2::TokenStream::new();
            for stmt in &input_fn.block.stmts {
                stmt.to_tokens(&mut ts);
            }
            ts
        };

        if input_fn.block.stmts.len() == 1 {
            match &input_fn.block.stmts[0] {
                syn::Stmt::Expr(expr, _) => {
                    if let syn::Expr::Macro(expr_macro) = expr {
                        if expr_macro.mac.path.is_ident("rsx") {
                            let mut iter = expr_macro.mac.tokens.clone().into_iter();
                            if let Some(proc_macro2::TokenTree::Group(group)) = iter.next() {
                                group.stream()
                            } else {
                                expr_macro.mac.tokens.clone()
                            }
                        } else {
                            build_fallback()
                        }
                    } else {
                        build_fallback()
                    }
                }
                _ => build_fallback(),
            }
        } else {
            build_fallback()
        }
    };

    // Generate the new function body modelled after TestBevyComponent
    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            use dioxus::prelude::*;
            use dioxus_in_bevy::prelude::*;
            use bevy::prelude::*;
            #[cfg(feature = "web")]
            use gloo_timers::future::TimeoutFuture;

            let world = use_bevy_world();
            let parent = use_bevy_parent();
            let mut _parent = use_signal(|| BevyParent::new(None));

            let entity = use_resource({
                move || async move {
                    loop {
                        if let Some(ref world) = *world.read() {
                            let entity = world.clone().spawn_empty().await.id();

                            if let Some(parent) = *parent() {
                                world.clone().entity(entity).insert(ChildOf(parent)).await;
                            }

                            _parent.set(BevyParent::new(Some(entity)));

                            return entity;
                        }

                        #[cfg(feature = "web")]
                        {
                            TimeoutFuture::new(16).await;
                        }
                        #[cfg(not(feature = "web"))]
                        {
                            // Yield execution on native platforms
                            std::thread::sleep(std::time::Duration::from_millis(16));
                        }
                    }
                }
            });

            use_context_provider(move || _parent);

            use_drop(move || {
                if let Some(world) = world() {
                    if let Some(entity) = entity() {
                        spawn_forever(async move {
                            world.entity(entity).despawn().await;
                        });
                    }
                }
            });

            if world.read().is_some() {
                #user_tokens
            } else {
                Ok(VNode::placeholder())
            }
        }
    };

    expanded.into()
}
