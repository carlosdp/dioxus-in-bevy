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
        .add_plugins(DioxusPlugin::default())
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

#[bevy_component]
fn TestBevyContainer(children: Element) -> Element {
    use_effect({
        move || {
            if let Some(entity) = entity() {
                spawn(async move {
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
                });
            }
        }
    });

    rsx! {
        {children}
    }
}

#[bevy_component]
fn TestBevyComponent(children: Element) -> Element {
    use_effect({
        move || {
            if let Some(entity) = entity() {
                spawn(async move {
                    world()
                        .unwrap()
                        .entity(entity)
                        .insert(Text::new("HELLO WORLD"))
                        .await;
                });
            }
        }
    });

    rsx! {
        {children}
    }
}
