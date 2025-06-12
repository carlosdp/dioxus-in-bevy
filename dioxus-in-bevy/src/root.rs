use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex},
};

use bevy::prelude::*;
use bevy_async_ecs::AsyncWorld;
use dioxus::prelude::*;

#[cfg(feature = "web")]
use crate::web_node::Overlay;

pub(crate) fn setup_plugin(app: &mut App) {
    app.add_systems(Update, synchronize_components);
}

pub type ComponentMap = HashMap<Entity, DioxusNode>;

pub struct DioxusRoot {
    pub(crate) components: Arc<Mutex<Option<Signal<ComponentMap>>>>,
}

#[derive(Clone, Component)]
pub struct DioxusNode {
    pub component: Arc<dyn (Fn() -> Element) + Send + Sync + 'static>,
}

impl DioxusNode {
    pub fn new(component: impl Fn() -> Element + Send + Sync + 'static) -> Self {
        Self {
            component: Arc::new(component),
        }
    }
}

#[derive(Default, Clone, PartialEq)]
pub struct BevyParent {
    pub parent: Option<Entity>,
}

impl BevyParent {
    pub fn new(parent: Option<Entity>) -> Self {
        Self { parent }
    }
}

impl Deref for BevyParent {
    type Target = Option<Entity>;

    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}

#[component]
pub fn BevyApp(builder: Option<Box<fn() -> App>>, children: Element) -> Element {
    let mut async_world = use_signal::<Option<AsyncWorld>>(|| None);
    let components = use_signal::<ComponentMap>(HashMap::new);

    let parent = use_signal(BevyParent::default);

    use_context_provider(|| parent);
    use_context_provider(|| components);
    use_context_provider(|| async_world);

    let mount_bevy = move |_e: dioxus::prelude::Event<MountedData>| {
        let builder = builder.clone();
        async move {
            #[cfg(feature = "web")]
            {
                let mut bevy_app = if let Some(builder) = builder {
                    builder()
                } else {
                    App::new()
                };

                async_world.set(Some(AsyncWorld::from_world(bevy_app.world_mut())));

                bevy_app.insert_non_send_resource(DioxusRoot {
                    components: Arc::new(Mutex::new(Some(components))),
                });

                bevy_app.run();
            }
        }
    };

    rsx! {
        div { style: "width: 100%; height: 100%; position: absolute; top: 0; left: 0;",
            canvas {
                id: "bevy",
                onmounted: mount_bevy,
                style: "width: 100%; height: 100%;",
            }

            Overlay {}

            div { style: "width: 100%; height: 100%; position: absolute; top: 0; left: 0; pointer-events: none;",
                {children}
            }
        }
    }
}

fn synchronize_components(
    root: NonSendMut<DioxusRoot>,
    added: Query<(Entity, &DioxusNode), Added<DioxusNode>>,
    mut removed: RemovedComponents<DioxusNode>,
) {
    for entity in removed.read() {
        root.components
            .lock()
            .unwrap()
            .unwrap()
            .write()
            .remove(&entity);
    }

    for (entity, component) in &added {
        root.components
            .lock()
            .unwrap()
            .unwrap()
            .write()
            .insert(entity, component.clone());
    }
}
