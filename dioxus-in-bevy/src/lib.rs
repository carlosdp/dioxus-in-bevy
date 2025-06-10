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

/// Spawn a future on the appropriate runtime without blocking the current task.
///
/// On WebAssembly targets this uses `wasm_bindgen_futures::spawn_local`. On
/// native targets it spawns a new OS thread and blocks on the future. This keeps
/// the implementation lightweight without pulling in an async runtime
/// dependency.
#[cfg(target_arch = "wasm32")]
pub fn spawn_detached(fut: impl std::future::Future<Output = ()> + 'static) {
    use wasm_bindgen_futures::spawn_local;
    spawn_local(fut);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_detached(fut: impl std::future::Future<Output = ()> + 'static) {
    tokio::task::spawn_local(fut);
}
