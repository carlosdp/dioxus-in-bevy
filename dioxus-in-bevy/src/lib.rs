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
    pub use dioxus_in_bevy_macros::bevy_component;
    pub use dioxus_in_bevy_macros::create_all_elements;
}

pub use dioxus_in_bevy_macros::bevy_component;
pub use dioxus_in_bevy_macros::dioxus_elements;
pub use paste;

use bevy::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;

// See [inventory](https://docs.rs/inventory/latest/inventory/#webassembly-and-constructors)
#[cfg(target_family = "wasm")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

#[derive(Default)]
pub struct DioxusPlugin {
    enable_overlay: bool,
}

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        // See [inventory](https://docs.rs/inventory/latest/inventory/#webassembly-and-constructors)
        #[cfg(target_family = "wasm")]
        unsafe {
            __wasm_call_ctors();
        }

        app.add_plugins(AsyncEcsPlugin)
            .add_plugins(native::setup_plugin)
            .add_plugins(root::setup_plugin);

        #[cfg(feature = "web")]
        {
            if self.enable_overlay {
                app.add_systems(Startup, web_node::setup_web_overlay);
            }
        }
    }
}
