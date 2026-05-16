use smithay_client_toolkit::{
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{BTN_LEFT, CursorIcon, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::WaylandSurface,
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_keyboard, wl_pointer, wl_surface},
};

use crate::{app::AppState, model::Msg};

impl KeyboardHandler for AppState {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        if self.layer.wl_surface() == surface {
            self.keyboard_focus = true;
        }
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if self.layer.wl_surface() == surface {
            self.keyboard_focus = false;
        }
    }

    fn press_key(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        if let Some(msg) = key_event_to_msg(&event) {
            self.dispatch(qh, msg);
        }
    }

    fn repeat_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        self.press_key(conn, qh, keyboard, serial, event);
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }
}

impl PointerHandler for AppState {
    fn pointer_frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }

            let (x, y) = event.position;

            match event.kind {
                PointerEventKind::Enter { .. } => {
                    self.update_cursor_for_position(conn, x, y);
                    self.dispatch(qh, Msg::HoverAt { x, y });
                }
                PointerEventKind::Motion { .. } => {
                    self.update_cursor_for_position(conn, x, y);
                    self.dispatch(qh, Msg::HoverAt { x, y });
                }
                PointerEventKind::Leave { .. } => {
                    self.dispatch(qh, Msg::ClearHover);
                }
                PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                    self.update_cursor_for_position(conn, x, y);
                    self.dispatch(qh, Msg::PointerPressedAt { x, y });
                }
                _ => {}
            }
        }
    }
}

impl AppState {
    fn update_cursor_for_position(&mut self, conn: &Connection, x: f64, y: f64) {
        let layout = self.model.layout();

        let icon = if layout.search.contains(x, y) {
            CursorIcon::Text
        } else {
            CursorIcon::Default
        };

        self.set_cursor_icon(conn, icon);
    }

    fn set_cursor_icon(&mut self, conn: &Connection, icon: CursorIcon) {
        let Some(pointer) = self.themed_pointer.as_mut() else {
            return;
        };

        if let Err(err) = pointer.set_cursor(conn, icon) {
            log::debug!("no se pudo setear cursor {:?}: {err:?}", icon);
        }
    }
}

pub(super) fn key_event_to_msg(event: &KeyEvent) -> Option<Msg> {
    match event.keysym {
        Keysym::Escape => return Some(Msg::Quit),
        Keysym::Return => return Some(Msg::LaunchSelected),
        Keysym::BackSpace => return Some(Msg::Backspace),
        Keysym::Up => return Some(Msg::SelectPrev),
        Keysym::Down => return Some(Msg::SelectNext),
        _ => {}
    }

    let text = event.utf8.as_deref()?;
    let ch = text.chars().next()?;

    if text.chars().count() == 1 && !ch.is_control() {
        Some(Msg::Type(ch))
    } else {
        None
    }
}
