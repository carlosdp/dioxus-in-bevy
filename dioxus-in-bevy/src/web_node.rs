use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bevy::prelude::*;
use bevy_async_ecs::AsyncEcsPlugin;
use dioxus::prelude::*;

pub(crate) fn setup_plugin(app: &mut App) {
    let async_world = bevy_async_ecs::AsyncWorld::from_world(app.world_mut());

    app.add_plugins(AsyncEcsPlugin)
        .insert_resource(AsyncWorld(async_world))
        .init_non_send_resource::<DioxusWebRoot>()
        .add_systems(Startup, setup_web)
        .add_systems(Update, synchronize_web_components);
}

#[derive(Default, Clone)]
pub struct DioxusWebRoot {
    pub(crate) components: Arc<Mutex<Option<Signal<HashMap<Entity, WebNode>>>>>,
}

#[derive(Clone, Component)]
pub struct WebNode {
    pub component: Arc<dyn (Fn() -> Element) + Send + Sync + 'static>,
}

impl WebNode {
    pub fn new(component: impl Fn() -> Element + Send + Sync + 'static) -> Self {
        Self {
            component: Arc::new(component),
        }
    }
}

#[derive(Resource)]
pub struct AsyncWorld(pub bevy_async_ecs::AsyncWorld);

fn setup_web(
    web_root: NonSendMut<DioxusWebRoot>,
    windows: Query<&Window>,
    async_world: Res<AsyncWorld>,
) {
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
        "width: 100%; height: 100%; position: absolute; top: 0; left: 0; pointer-events: none;",
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
                width: "100%",
                height: "100%",
                position: "absolute",
                top: "0",
                left: "0",

                for component in components {
                    div {
                        width: "100%",
                        height: "100%",
                        position: "absolute",
                        top: "0",
                        left: "0",
                        display: "flex",

                        {component()}
                    }
                }
            }
        }
    });
    vdom.provide_root_context(web_root.clone());
    vdom.provide_root_context(async_world.0.clone());
    let components_signal: Signal<HashMap<Entity, WebNode>> =
        vdom.in_runtime(|| Signal::new_in_scope(HashMap::new(), ScopeId::ROOT));
    *web_root.components.lock().unwrap() = Some(components_signal);
    // note(carlos): I don't love this, and this sucks because we can't "shut down"
    // the UI really. But the web dom stuff is all private.
    dioxus_web::launch::launch_virtual_dom(vdom, config);
}

fn synchronize_web_components(
    root: NonSendMut<DioxusWebRoot>,
    added: Query<(Entity, &WebNode), Added<WebNode>>,
    mut removed: RemovedComponents<WebNode>,
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
