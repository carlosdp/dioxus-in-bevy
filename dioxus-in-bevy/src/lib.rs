pub use dioxus_core;
pub use generational_box;
pub use inventory;

pub mod component;
pub mod hooks;
pub mod macros;
#[cfg(feature = "web")]
pub mod web_node;

mod history;
pub mod native;
mod renderers;
mod root;

pub mod prelude {
    pub use super::DioxusPlugin;
    pub use crate::component::attr;
    pub use crate::hooks::*;
    pub use crate::macros::elements;
    pub use crate::macros::events;
    pub use crate::root::{BevyApp, BevyParent, DioxusNode};
    pub use dioxus_in_bevy_macros::create_all_elements;
}

pub use dioxus_in_bevy_macros::dioxus_elements;
pub use paste;

use bevy::prelude::*;

// See [inventory](https://docs.rs/inventory/latest/inventory/#webassembly-and-constructors)
#[cfg(target_family = "wasm")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

pub struct DioxusPlugin;

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        // See [inventory](https://docs.rs/inventory/latest/inventory/#webassembly-and-constructors)
        #[cfg(target_family = "wasm")]
        unsafe {
            __wasm_call_ctors();
        }

        app.add_plugins(native::setup_plugin)
            .add_plugins(root::setup_plugin);

        #[cfg(feature = "web")]
        {
            app.add_plugins(web_node::setup_plugin);
        }
    }
}
