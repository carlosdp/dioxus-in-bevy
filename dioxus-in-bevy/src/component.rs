use std::rc::Rc;

use bevy::ecs::entity::Entity;
use dioxus_core::{AnyValue, AttributeValue};

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct ComponentBuilder {
    pub name: &'static str,
    pub builder: fn(&mut bevy::prelude::World) -> Entity,
    pub initializer: fn(&mut bevy::prelude::World, Entity),
    pub attribute_mutators: &'static [(
        &'static str,
        fn(&mut bevy::prelude::World, Entity, dioxus_core::AttributeValue),
    )],
}

inventory::collect!(ComponentBuilder);

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct EventHandlerInstaller {
    pub name: &'static str,
    pub handler: fn(&mut bevy::prelude::World, Entity, Entity, dioxus_core::ElementId),
}

inventory::collect!(EventHandlerInstaller);

pub fn attr<T: AnyValue + Clone + 'static>(value: T) -> dioxus_core::AttributeValue {
    AttributeValue::Any(Rc::new(value))
}

#[doc(hidden)]
pub fn convert_attribute<T: AnyValue + Clone + 'static>(value: dioxus_core::AttributeValue) -> T {
    let type_name = std::any::type_name::<T>();

    if type_name == std::any::type_name::<String>() {
        match value {
            AttributeValue::Text(text) => text.as_any().downcast_ref::<T>().unwrap().clone(),
            _ => panic!("Attribute is not a String!"),
        }
    } else if type_name == std::any::type_name::<f64>() {
        match value {
            AttributeValue::Float(number) => number.as_any().downcast_ref::<T>().unwrap().clone(),
            _ => panic!("Attribute is not a number!"),
        }
    } else if type_name == std::any::type_name::<i64>() {
        match value {
            AttributeValue::Int(number) => number.as_any().downcast_ref::<T>().unwrap().clone(),
            _ => panic!("Attribute is not a number!"),
        }
    } else if type_name == std::any::type_name::<bool>() {
        match value {
            AttributeValue::Bool(bool) => bool.as_any().downcast_ref::<T>().unwrap().clone(),
            _ => panic!("Attribute is not a bool!"),
        }
    } else {
        match value {
            AttributeValue::Any(any) => match any.as_any().downcast_ref::<T>() {
                Some(value) => value.clone(),
                None => panic!("Attribute is not a {:?}!", std::any::type_name::<T>()),
            },
            _ => panic!("Attribute is not a {:?}!", std::any::type_name::<T>()),
        }
    }
}
