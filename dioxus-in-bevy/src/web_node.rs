use std::collections::HashMap;

use bevy::prelude::*;
use bevy_async_ecs::AsyncWorld;
use dioxus::prelude::*;

use crate::root::{ComponentMap, DioxusRoot};

pub(crate) fn setup_web_overlay(world: &mut World) {
    use dioxus::signals::Signal;
    use web_sys::window;

    let async_world = AsyncWorld::from_world(world);
    let mut windows = world.query::<&Window>();
    let bevy_window = windows.single(world).unwrap();
    let canvas_selector = bevy_window.canvas.clone();
    let web_root = world.non_send_resource_mut::<DioxusRoot>();

    let canvas = if let Some(ref canvas_selector) = canvas_selector {
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
    let vdom = VirtualDom::new(Overlay);
    vdom.provide_root_context(Some(async_world));
    let components_signal: Signal<ComponentMap> =
        vdom.in_runtime(|| Signal::new_in_scope(HashMap::new(), ScopeId::ROOT));
    vdom.provide_root_context(components_signal);
    *web_root.components.lock().unwrap() = Some(components_signal);
    // note(carlos): I don't love this, and this sucks because we can't "shut down"
    // the UI really. But the web dom stuff is all private.
    dioxus_web::launch::launch_virtual_dom(vdom, config);
}

#[component]
pub fn Overlay() -> Element {
    let components = use_context::<Signal<ComponentMap>>();

    let components = components
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
            pointer_events: "none",
            id: "overlay",

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
}
