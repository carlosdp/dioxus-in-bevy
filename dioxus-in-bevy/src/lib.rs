pub use dioxus_core;
pub use generational_box;
pub use inventory;

pub mod component;
pub mod macros;

mod history;
mod renderers;

pub mod prelude {
    #[cfg(feature = "web")]
    pub use super::WebComponent;
    pub use super::{DioxusPlugin, DioxusRoot};
    pub use crate::component::attr;
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
    sync::{Arc, Mutex},
};

use bevy::prelude::*;
use component::ComponentBuilder;
#[cfg(feature = "web")]
use dioxus::prelude::*;
use dioxus_core::{Element, ScopeId, VirtualDom};

thread_local! {
    pub static RENDERER_CONTEXT: RefCell<DioxusRendererContext> = RefCell::new(DioxusRendererContext::default());
}

#[derive(Component)]
pub struct DioxusRoot {
    pub(crate) root: fn() -> Element,
    pub(crate) element_map: HashMap<dioxus_core::ElementId, Entity>,
}

#[cfg(feature = "web")]
#[derive(Default, Clone)]
pub struct DioxusWebRoot {
    pub(crate) components: Arc<Mutex<Option<Signal<HashMap<Entity, WebComponent>>>>>,
}

#[cfg(feature = "web")]
#[derive(Clone, Component)]
#[require(Node)]
pub struct WebComponent {
    pub component: Arc<dyn (Fn() -> Element) + Send + Sync + 'static>,
}

impl WebComponent {
    pub fn new(component: impl Fn() -> Element + Send + Sync + 'static) -> Self {
        Self {
            component: Arc::new(component),
        }
    }
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
            app.init_non_send_resource::<DioxusWebRoot>()
                .add_systems(Startup, setup_web)
                .add_systems(Update, synchronize_web_components);
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

#[cfg(feature = "web")]
fn setup_web(web_root: NonSendMut<DioxusWebRoot>, windows: Query<&Window>) {
    use dioxus::signals::Signal;
    use web_sys::window;

    let bevy_window = windows.single().unwrap();
    let canvas = if let Some(ref canvas_selector) = bevy_window.canvas {
        window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&canvas_selector)
            .expect("Failed to query canvas selector")
            .unwrap()
    } else {
        window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("canvas")
            .expect("Failed to query canvas selector")
            .unwrap()
    };
    let ui_root = canvas.parent_node().unwrap();
    let node = window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    node.set_attribute(
        "style",
        "width: 100%; height: 100%; position: absolute; top: 0; left: 0;",
    )
    .unwrap();
    ui_root.append_child(&node).unwrap();

    let config = dioxus_web::Config::new().rootelement(node);
    let vdom = VirtualDom::new(move || {
        let web_root = use_context::<DioxusWebRoot>();

        let components = web_root
            .components
            .lock()
            .unwrap()
            .unwrap()
            .read()
            .iter()
            .map(|(_, comp)| comp.component.clone())
            .collect::<Vec<_>>();

        rsx! {
            div {
                for component in components {
                    {component()}
                }
            }
        }
    });
    vdom.provide_root_context(web_root.clone());
    let components_signal: Signal<HashMap<Entity, WebComponent>> =
        vdom.in_runtime(|| Signal::new_in_scope(HashMap::new(), ScopeId::ROOT));
    *web_root.components.lock().unwrap() = Some(components_signal);
    // note(carlos): I don't love this, and this sucks because we can't "shut down"
    // the UI really. But the web dom stuff is all private.
    dioxus_web::launch::launch_virtual_dom(vdom, config);
}

fn synchronize_web_components(
    root: NonSendMut<DioxusWebRoot>,
    added: Query<(Entity, &WebComponent), Added<WebComponent>>,
    mut removed: RemovedComponents<WebComponent>,
) {
    for (entity, component) in &added {
        root.components
            .lock()
            .unwrap()
            .unwrap()
            .write()
            .insert(entity, component.clone());
    }

    for entity in removed.read() {
        root.components
            .lock()
            .unwrap()
            .unwrap()
            .write()
            .remove(&entity);
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
