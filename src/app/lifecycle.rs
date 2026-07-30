use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, Keysym},
        pointer::ThemeSpec,
    },
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
    shm::{Shm, ShmHandler},
};

use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_keyboard, wl_output, wl_seat, wl_surface},
};

use super::input::{KEY_REPEAT_STEPS, key_event_to_msg};

use crate::{app::AppState, model::Msg};

impl CompositorHandler for AppState {
    fn scale_factor_changed(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, surface: &wl_surface::WlSurface, new_factor: i32) {
        if self.layer.wl_surface() == surface {
            self.dispatch(qh, Msg::ScaleChanged(new_factor));
        }
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _time: u32) {
        self.redraw_scheduled = false;
        self.render_now(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for AppState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for AppState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.should_close = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (w, h) = configure.new_size;

        self.dispatch(
            qh,
            Msg::Configured {
                width: w.max(1),
                height: h.max(1),
            },
        );
    }
}

impl SeatHandler for AppState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let repeat_qh = qh.clone();

            let callback = Box::new(move |app: &mut AppState, _keyboard: &wl_keyboard::WlKeyboard, event: KeyEvent| match event.keysym {
                Keysym::Up => {
                    for _ in 0..KEY_REPEAT_STEPS {
                        app.dispatch(&repeat_qh, Msg::SelectPrev);
                    }
                }
                Keysym::Down => {
                    for _ in 0..KEY_REPEAT_STEPS {
                        app.dispatch(&repeat_qh, Msg::SelectNext);
                    }
                }
                _ => {
                    if let Some(msg) = key_event_to_msg(&event) {
                        app.dispatch(&repeat_qh, msg);
                    }
                }
            });

            match self
                .seat_state
                .get_keyboard_with_repeat(qh, &seat, None, self.loop_handle.clone(), callback)
            {
                Ok(keyboard) => self.keyboard = Some(keyboard),
                Err(err) => log::warn!("no se pudo crear keyboard con repeat: {err:?}"),
            }
        }

        if capability == Capability::Pointer && self.themed_pointer.is_none() {
            let surface = self.compositor.create_surface(qh);

            match self
                .seat_state
                .get_pointer_with_theme(qh, &seat, self.shm.wl_shm(), surface, ThemeSpec::default())
            {
                Ok(pointer) => self.themed_pointer = Some(pointer),
                Err(err) => log::warn!("no se pudo crear themed pointer: {err:?}"),
            }
        }
    }

    fn remove_capability(&mut self, _conn: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            keyboard.release();
        }

        if capability == Capability::Pointer
            && let Some(pointer) = self.themed_pointer.take()
        {
            pointer.pointer().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ShmHandler for AppState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(AppState);
delegate_output!(AppState);
delegate_shm!(AppState);
delegate_seat!(AppState);
delegate_keyboard!(AppState);
delegate_pointer!(AppState);
delegate_layer!(AppState);
delegate_registry!(AppState);
