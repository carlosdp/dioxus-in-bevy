use bevy::prelude::*;
use dioxus::prelude::*;
use dioxus_in_bevy::prelude::*;

fn main() {
    dioxus_web::launch::launch(App, Vec::new(), Vec::new());
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    commands.spawn(DioxusNode::new(TestWebComponent));
}

fn bevy_app() -> bevy::prelude::App {
    let mut app = bevy::prelude::App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin)
        .add_systems(Startup, setup);
    app
}

#[component]
fn App() -> Element {
    rsx! {
        BevyApp { builder: Some(Box::new(bevy_app as fn() -> bevy::prelude::App)),
            TestWebComponent {}

            SuspenseBoundary { fallback: |_| rsx! {},
                TestBevyContainer { TestBevyComponent {} }
            }
        }
    }
}

#[component]
fn TestWebComponent() -> Element {
    rsx! {
        div { "Hello, world!" }
    }
}

#[component]
fn TestBevyContainer(children: Element) -> Element {
    let world = use_bevy_world();
    let parent = use_bevy_parent();
    let entity = use_resource({
        move || async move {
            loop {
                if let Some(ref world) = *world.read() {
                    let entity = world.clone().spawn_empty().await.id();

                    if let Some(parent) = parent.parent {
                        world.clone().entity(entity).insert(ChildOf(parent)).await;
                    }

                    return entity;
                }

                gloo_timers::future::TimeoutFuture::new(16).await;
            }
        }
    })
    .suspend()?;
    let entity = entity.cloned();

    use_context_provider(move || BevyParent::new(entity));

    use_effect({
        move || {
            spawn_detached(async move {
                world()
                    .unwrap()
                    .entity(entity)
                    .insert((
                        Node {
                            width: Val::Px(100.0),
                            height: Val::Px(100.0),
                            ..Default::default()
                        },
                        BackgroundColor(Color::BLACK),
                    ))
                    .await;
            })
        }
    });

    use_drop(move || {
        spawn_detached(async move {
            if let Some(ref world) = *world.read() {
                world.entity(entity).despawn().await;
            }
        })
    });

    rsx! {
        {children}
    }
}

#[component]
fn TestBevyComponent(children: Element) -> Element {
    let world = use_bevy_world();
    let parent = use_bevy_parent();
    let entity = use_resource({
        move || async move {
            loop {
                if let Some(ref world) = *world.read() {
                    let entity = world.clone().spawn_empty().await.id();

                    if let Some(parent) = parent.parent {
                        world.clone().entity(entity).insert(ChildOf(parent)).await;
                    }

                    return entity;
                }

                gloo_timers::future::TimeoutFuture::new(16).await;
            }
        }
    })
    .suspend()?;
    let entity = entity.cloned();

    use_context_provider(move || BevyParent::new(entity));

    use_effect({
        move || {
            spawn_detached(async move {
                world()
                    .unwrap()
                    .entity(entity)
                    .insert(Text::new("HELLO WORLD"))
                    .await;
            })
        }
    });

    use_drop(move || {
        spawn_detached(async move {
            if let Some(ref world) = *world.read() {
                world.entity(entity).despawn().await;
            }
        })
    });

    rsx! {
        {children}
    }
}

// spawn.rs
// #[cfg(target_arch = "wasm32")]
pub fn spawn_detached(fut: impl std::future::Future<Output = ()> + 'static) {
    use wasm_bindgen_futures::spawn_local;
    spawn_local(fut);
}

// #[cfg(not(target_arch = "wasm32"))]
// pub fn spawn_detached(fut: impl std::future::Future<Output = ()> + Send + 'static) {
//     tokio::spawn(fut);
// }
