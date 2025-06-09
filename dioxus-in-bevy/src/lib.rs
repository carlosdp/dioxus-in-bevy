pub use dioxus_core;
pub use generational_box;
pub use inventory;

pub mod component;
pub mod hooks;
pub mod macros;
#[cfg(feature = "web")]
pub mod web_node;

mod history;
mod renderers;

pub mod prelude {
    #[cfg(feature = "web")]
    pub use super::web_node::WebNode;
    pub use super::{DioxusPlugin, DioxusRoot};
    pub use crate::component::attr;
    pub use crate::hooks::*;
    pub use crate::macros::elements;
    pub use crate::macros::events;
    pub use dioxus_in_bevy_macros::create_all_elements;
}

pub use dioxus_in_bevy_macros::dioxus_elements;

pub use paste;
use renderers::WorldRenderer;

use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use bevy::prelude::*;
use component::ComponentBuilder;
#[cfg(feature = "web")]
use dioxus::prelude::*;
use dioxus_core::{Element, ScopeId, VirtualDom};

thread_local! {
    pub static RENDERER_CONTEXT: RefCell<DioxusRendererContext> = RefCell::new(DioxusRendererContext::default());
}

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

        let builders =
            HashMap::from_iter(inventory::iter::<ComponentBuilder>().map(|b| (b.name, b)));

        app.init_non_send_resource::<DioxusCommands>()
            .init_non_send_resource::<EventChannels>()
            .insert_resource(DioxusBuilders(builders))
            .add_systems(Update, (setup, render, process_commands));

        #[cfg(feature = "web")]
        {
            app.add_plugins(web_node::setup_plugin);
        }
    }
}

#[derive(Component)]
pub struct DioxusRoot {
    pub(crate) root: fn() -> Element,
    pub(crate) element_map: HashMap<dioxus_core::ElementId, Entity>,
}

impl DioxusRoot {
    pub fn new(root: fn() -> Element) -> Self {
        Self {
            root,
            element_map: HashMap::new(),
        }
    }
}

#[derive(Default)]
pub struct DioxusRendererContext {
    pub renderers: HashMap<Entity, (VirtualDom, WorldRenderer)>,
}

#[derive(Resource, Default)]
pub struct DioxusBuilders(pub HashMap<&'static str, &'static ComponentBuilder>);

impl Deref for DioxusBuilders {
    type Target = HashMap<&'static str, &'static ComponentBuilder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct DioxusCommands(pub Vec<Box<dyn FnOnce(&mut World) + 'static>>);

impl std::fmt::Debug for DioxusCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DioxusCommands({})", self.0.len())
    }
}

impl Deref for DioxusCommands {
    type Target = Vec<Box<dyn FnOnce(&mut World) + 'static>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DioxusCommands {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Component)]
pub struct ElementTag(pub &'static str);

#[derive(Default)]
pub struct EventChannels {
    pub channels: HashMap<
        Entity,
        HashMap<
            &'static str,
            dioxus_core::prelude::EventHandler<dioxus_core::prelude::Event<dyn Any>>,
        >,
    >,
}

impl std::fmt::Debug for EventChannels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventChannels({})", self.channels.len())
    }
}

fn init_history() {
    if ScopeId::ROOT
        .has_context::<Rc<dyn dioxus::prelude::document::Document>>()
        .is_none()
    {
        #[cfg(feature = "web")]
        {
            let history_provider: Rc<dyn dioxus::prelude::document::Document> =
                Rc::new(dioxus_web::WebDocument);
            ScopeId::ROOT.provide_context(history_provider);
        }
    }

    if ScopeId::ROOT
        .has_context::<Rc<dyn dioxus::prelude::History>>()
        .is_none()
    {
        #[cfg(feature = "web")]
        {
            let history_provider: Rc<dyn dioxus::prelude::History> =
                Rc::new(history::web::WebHistory::default());
            ScopeId::ROOT.provide_context(history_provider);
        }
    }
}

fn setup(
    mut query: Query<(Entity, &mut DioxusRoot), Added<DioxusRoot>>,
    mut commands: Commands,
    mut dioxus_commands: NonSendMut<DioxusCommands>,
) {
    for (entity, mut dioxus_root) in query.iter_mut() {
        let root = dioxus_root.root;

        dioxus_root
            .element_map
            .insert(dioxus_core::ElementId(0), entity);
        commands.entity(entity).insert(Node::default());

        let mut vdom = VirtualDom::new(root);
        vdom.in_runtime(init_history);
        let mut renderer = WorldRenderer::new(entity);

        vdom.rebuild(&mut renderer);

        dioxus_commands.extend(renderer.drain_commands());

        RENDERER_CONTEXT.with_borrow_mut(|context| {
            context.renderers.insert(entity, (vdom, renderer));
        });
    }
}

fn render(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<DioxusRoot>>();
    let mut commands_to_run = Vec::new();
    for entity in query.iter(world) {
        RENDERER_CONTEXT.with_borrow_mut(|context| {
            if let Some((vdom, renderer)) = context.renderers.get_mut(&entity) {
                if let Some(_) = futures_lite::future::block_on(futures_lite::future::poll_once(
                    vdom.wait_for_work(),
                )) {
                    vdom.render_immediate(renderer);
                    let commands = renderer.drain_commands();
                    commands_to_run.extend(commands);
                }
            }
        });
    }

    world
        .get_non_send_resource_mut::<DioxusCommands>()
        .unwrap()
        .extend(commands_to_run);
}

fn process_commands(world: &mut World) {
    let mut dioxus_commands = world.get_non_send_resource_mut::<DioxusCommands>().unwrap();

    let commands = dioxus_commands.drain(..).collect::<Vec<_>>();

    for command in commands {
        command(world);
    }
}
