use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::component::ComponentBuilder;
use crate::renderers::WorldRenderer;
use bevy::prelude::*;
#[cfg(feature = "web")]
use dioxus::prelude::*;
use dioxus_core::{Element, ScopeId, VirtualDom};

thread_local! {
    pub static RENDERER_CONTEXT: RefCell<DioxusRendererContext> = RefCell::new(DioxusRendererContext::default());
}

pub(crate) fn setup_plugin(app: &mut App) {
    let builders = HashMap::from_iter(inventory::iter::<ComponentBuilder>().map(|b| (b.name, b)));

    app.init_non_send_resource::<DioxusCommands>()
        .init_non_send_resource::<EventChannels>()
        .insert_resource(DioxusBuilders(builders))
        .add_systems(Update, (setup, render, process_commands));
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
                Rc::new(crate::history::web::WebHistory::default());
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
