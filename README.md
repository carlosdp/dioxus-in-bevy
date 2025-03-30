# dioxus-in-bevy

⚠️ **HIGHLY EXPERIMENTAL AND UNSTABLE** ⚠️

An experimental integration that enables the use of Dioxus as a reactive UI engine for Bevy applications.

## Status

This crate is in a very early experimental stage and is **NOT** recommended for production use. Expect:
- Frequent breaking changes
- Missing features
- Performance issues
- Limited documentation
- API instability

It exists in public at the moment to solicit input from developers.

## Purpose

Dioxus-in-Bevy aims to bring Dioxus's reactive component model to Bevy applications, allowing developers to:
- Build Bevy UI components using Dioxus's declarative React-like syntax
- Leverage Dioxus's reactive state management with Bevy's ECS

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
dioxus-in-bevy = { git = "https://github.com/carlosdp/dioxus-in-bevy.git" }
dioxus = "0.6"
bevy = "0.15"
```

## Basic Usage

```rust
use bevy::prelude::*;
use dioxus::prelude::*;
use dioxus_in_bevy::prelude::*;

// Define custom elements that map to Bevy components
dioxus_in_bevy::elements! {
    custom_elements:

    node, (Node::default(), Interaction::default()), {
        background_color: Color {
            (background_color_node: &mut BackgroundColor) => {
                background_color_node.0 = background_color;
            }
        }
    },
    text, Text::default(), {
        text: String {
            (text_node: &mut Text) => {
                text_node.0 = text;
            }
        }
    },
}

// Define custom events
dioxus_in_bevy::events! {
    custom_events:

    onclick: |_trigger: Trigger<Pointer<Click>>| -> () {}
}

// Register elements and events
dioxus_in_bevy::dioxus_elements! {
    elements: { custom_elements }
    events: { custom_events }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    // Spawn the Dioxus app in the World
    commands.spawn(DioxusRoot::new(App));
}

#[component]
fn App() -> Element {
    rsx! {
        node { background_color: attr(Color::srgb(1.0, 0.0, 0.0)),
            text {
                text: "Hello, world!",
                onclick: |_| {
                    println!("onclick triggered");
                }
            }
        }
    }
}
```

### Defining Elements

The Dioxus `rsx! {}` macro expects elements (in HTML, this would be `div`, `a`, etc.) to exist in scope in a particular format under the module name `dioxus_elements`. By default, `use dioxus::prelude::*` imports the default HTML/SVG `dioxus_elements` module. In order to support Bevy components/entities, we need a way to define our elements as bundles, and tell Dioxus how translate Dioxus element attributes into Bevy component mutations. We also need to be able to compose multiple modules of these elements into our own `dioxus_elements` module. For this, the crate has several macros:

#### `elements!`

You can define custom elements in a module using the `elements!` macro. This takes a name for the resulting module (here it's `my_elements`), and then a series of element definitions containing:

- Element name (ie. `node`, `text`)
- A valid Bevy Bundle to spawn the initial entity
- A series of setters that take an attribute passed into Dioxus, and perform a mutation on the entity

```rust
dioxus_in_bevy::elements! {
    my_elements:

    node, (Node::default(), Interaction::default()), {
        background_color: Color {
            (background_color_node: &mut BackgroundColor) => {
                background_color_node.0 = background_color;
            }
        }
    },
    text, Text::default(), {
        text: String {
            (text_node: &mut Text) => {
                text_node.0 = text;
            }
        }
    },
}
```

##### Attribute Setters
An attribute setter consists of:

- The attribute name (ie. `background_color`)
- The input type to accept (if the type is not a standard Rust type, or otherwise not a type that has built-in support from Dioxus, you have to wrap the value in the `attr()` function in the crate. This simply wraps the attribute in a type-erased `Rc`)
- A closure that takes component Query parameters (anything you could put in a `Query<(...)>` in Bevy), and has access to the current value of the attribute by its name, so you can mutate components as needed to synchronize the state. These will be called whenever Dioxus detects a change that triggers a new attribute value.

You can also access arbitrary `SystemParam`s from the ECS world by using this special syntax:

```rust
dioxus_in_bevy::elements! {
    my_elements:

    node, (Node::default(), Interaction::default()), {
        background_color: Color {
            // anything in [ ] brackets will be queried from the ECS World via `IntoSystem`
            (background_color_node: &mut BackgroundColor)[colors: ResMut<ColorMap>] => {
                colors.add(background_color.clone());
                background_color_node.0 = background_color;
            }
        }
    },
}
```

#### `events!`

The `events!` macro works in a similar fashion, in order to provide Dioxus event handlers via Bevy Observers. Each event definition contains:

- The event name (ie. `onclick`) (note: event names MUST look like web events do, they can't have underscores, special characters, etc. They don't _have_ to start with "on", but the Dioxus autocomplete works better if they do)
- A closure that takes the same input for an Observer system (first param is the Trigger, subsequent is any valid `SystemParam`s), and returns a value that will be passed into the event handler defined in the Dioxus rsx as an `Event`.

```rust
dioxus_in_bevy::events! {
    my_events:

    onchange: |trigger: Trigger<TextInputChanged>| -> String {
      trigger.event().value.clone()
    }
}

// May be used like this
rsx! {
  //...
  textinput {
    onchange: move |ev| {
      println!("Current value: {}", ev.data());
    }
  }
}
```

### A note on HTML

Dioxus has been hard at work on their native renderer, [Blitz](https://github.com/DioxusLabs/blitz), which is a lightweight, modular web rendering engine that uses `wgpu`. I believe it might be possible, with some work, to create a mechanism where we can create a special fragment element that renders HTML/SVG in Dioxus via the Bevy rendering engine to a surface, managed by a Bevy Node, for example.

## Future Ideas

- [ ] Support Hot Reloading (this shouldn't actually be that difficult, given how the latest version of Dixous hot-reloading is implemented)
- [ ] Support HTML/SVG via an embedded Blitz renderer
- [ ] A default element set for Bevy user interface components

## Examples

Check the `examples/` directory for more detailed examples:
- `blank.rs`: Basic setup with custom elements and events
- `using_signals.rs`: Using Dioxus signals for reactive state

## Contributing

Contributions welcome! Just open a PR 🙂

## License

This crate is dual licensed with MIT or Apache 2.0, your choice!
