use proc_macro::TokenStream;
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
