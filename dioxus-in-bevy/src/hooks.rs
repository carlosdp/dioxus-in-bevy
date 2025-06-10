use bevy::ecs::system::{In, IntoSystem};
use bevy_async_ecs::AsyncWorld;
use dioxus::prelude::*;

use crate::root::BevyParent;

pub fn use_bevy_world() -> Signal<Option<AsyncWorld>> {
    use_context::<Signal<Option<AsyncWorld>>>()
}

// TODO: I don't love this approach. We should re-engineer so we don't need the unecessary In<>,
// (a limitation of bevy_async_ecs), and we use the actual Update schedule instead of manually looping.
// Also, we need to add the loop delay for non-web targets.
pub fn use_bevy_update<
    O: Send + 'static,
    M,
    S: IntoSystem<In<()>, Option<O>, M> + Clone + Send + 'static,
>(
    system: S,
) -> Signal<Option<O>> {
    let world = use_bevy_world();
    let mut signal = use_signal::<Option<O>>(|| None);

    use_future({
        let world = world.clone();
        move || {
            let world = world.clone();
            let system = system.clone();
            async move {
                if let Some(ref world) = *world.read() {
                    let sys = world.register_io_system(system).await;

                    loop {
                        let out = sys.run(()).await;

                        if out.is_some() {
                            signal.replace(out);
                        }

                        #[cfg(feature = "web")]
                        gloo_timers::future::TimeoutFuture::new(16).await;
                    }
                }
            }
        }
    });

    signal
}

pub fn use_bevy_parent() -> BevyParent {
    use_context::<BevyParent>()
}
