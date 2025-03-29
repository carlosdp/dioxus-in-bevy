#[macro_export]
macro_rules! elements {
    (
        $module_name:ident:

        $($component_name:ident, $bundle_expr:expr, {
            $(@init: $(<$init_self_ident:ident>)? ($($init_param_name:ident: $init_param_type:ty$(,)?)+) $([$($init_world_params:tt)*])? => { $($init_attr_body:tt)* })?$(,)?
            $(
                $attr_name:ident: $attr_type:ty {
                   $(<$self_ident:ident>)? ($($param_name:ident: $param_type:ty$(,)?)+) $([$($world_params:tt)*])? => { $($attr_body:tt)* }
                }$(,)?
            )*
        }$(,)?)*
    ) => {
        pub mod $module_name {
            use super::*;

            pub use elements::*;

            pub mod elements {
                pub use super::*;

                $(
                    #[allow(non_camel_case_types)]
                    #[allow(non_upper_case_globals)]
                    pub mod $component_name {
                        pub use super::*;

                        pub const TAG_NAME: &'static str = stringify!($component_name);
                        pub const NAME_SPACE: Option<&'static str> = None;

                        $(
                            pub const $attr_name: (&'static str, Option<&'static str>, bool) = (stringify!($attr_name), None, false);
                        )*
                    }

                    $crate::inventory::submit! {
                        $crate::component::ComponentBuilder {
                            name: stringify!($component_name),
                            builder: |world| {
                                world.spawn($bundle_expr).id()
                            },
                            initializer: |world: &mut bevy::prelude::World, entity: bevy::ecs::entity::Entity| {
                                $(
                                    let system_id = world.register_system(move |mut query: bevy::prelude::Query<($($init_param_type),*)>$(, $($init_world_params)*)?| {
                                        let ($(mut $init_param_name),*) = query.get_mut(entity).unwrap();
                                        $(let $init_self_ident = entity;)?
                                        {
                                            $($init_attr_body)*
                                        }
                                    });
                                    world.run_system(system_id).expect("Initialization failed");
                                )?
                            },
                            attribute_mutators: &[
                                $(
                                    (stringify!($attr_name), |world: &mut bevy::prelude::World, entity: bevy::ecs::entity::Entity, value: $crate::dioxus_core::AttributeValue| {
                                        let system_id = world.register_system(move |bevy::prelude::In($attr_name): bevy::prelude::In<$attr_type>, mut query: bevy::prelude::Query<($($param_type),*)>$(, $($world_params)*)?| {
                                            let ($(mut $param_name),*) = query.get_mut(entity).unwrap();
                                            $(let $self_ident = entity;)?
                                            {
                                                $($attr_body)*
                                            }
                                        });
                                        let converted_value: $attr_type = $crate::component::convert_attribute(value);
                                        world.run_system_with_input(system_id, converted_value).expect("Attribute mutation failed");
                                    }),
                                )*
                            ],
                        }
                    }
                )*

                #[doc(hidden)]
                #[allow(unused)]
                pub mod completions {
                    #[allow(non_camel_case_types)]
                    pub enum CompleteWithBraces {
                        $(
                            $component_name {},
                        )*
                    }
                }
            }

            dioxus_in_bevy::prelude::create_all_elements! {
                $module_name
                $(
                    $component_name
                ),*
            }
        }
    };
}

#[macro_export]
macro_rules! events {
    (
        $module_name:ident:

        $(
            $event_name:ident: |$($event_param:ident: $event_type:ty$(,)?)+| -> $event_return:ty $event_body:block$(,)?
        )*
    ) => {
        pub mod $module_name {
            use super::*;

            $(
                #[inline]
                pub fn $event_name<__Marker>(
                    mut _f: impl $crate::dioxus_core::prelude::SuperInto<$crate::dioxus_core::prelude::EventHandler<$crate::dioxus_core::Event<$event_return>>, __Marker>
                ) -> $crate::dioxus_core::Attribute {
                    let owner = <$crate::generational_box::UnsyncStorage as $crate::generational_box::AnyStorage>::owner();
                    let event_handler = $crate::dioxus_core::prelude::with_owner(owner.clone(), || _f.super_into());

                    $crate::dioxus_core::Attribute::new(
                        stringify!($event_name),
                        $crate::dioxus_core::AttributeValue::listener(move |e: $crate::dioxus_core::Event<$event_return>| {
                            _ = &owner;
                            event_handler.call(e.map(|e| e.clone()));
                        }),
                        None,
                        false,
                    ).into()
                }

                #[doc(hidden)]
                pub mod $event_name {
                    use super::*;

                    pub fn call_with_explicit_closure<
                        __Marker,
                        Return: $crate::dioxus_core::SpawnIfAsync<__Marker> + 'static,
                        F: FnMut($crate::dioxus_core::Event<$event_return>) -> Return + 'static
                    >(
                        event_handler: F,
                    ) -> $crate::dioxus_core::Attribute {
                        #[allow(deprecated)]
                        super::$event_name(event_handler)
                    }
                }
            )*
        }

        $(
            $crate::inventory::submit! {
                $crate::component::EventHandlerInstaller {
                    name: stringify!($event_name),
                    handler: |
                        world: &mut bevy::prelude::World,
                        root_entity: bevy::ecs::entity::Entity,
                        entity: bevy::ecs::entity::Entity,
                        id: $crate::dioxus_core::ElementId
                    | {
                        world.entity_mut(entity).observe(move |$($event_param: $event_type),*, _windows: NonSend<bevy::winit::WinitWindows>| {
                            let event_return = { $event_body };
                            $crate::RENDERER_CONTEXT.with_borrow(|context| {
                                if let Some((vdom, _)) = context.renderers.get(&root_entity) {
                                    let runtime = vdom.runtime();
                                    let event_name = if stringify!($event_name).starts_with("on") { stringify!($event_name)[2..].to_string() } else { stringify!($event_name).to_string() };
                                    runtime.handle_event(&event_name, $crate::dioxus_core::Event::new(std::rc::Rc::new(event_return), true), id);
                                }
                            });
                        });
                    },
                }
            }
        )*
    };
}

pub use elements;
pub use events;
