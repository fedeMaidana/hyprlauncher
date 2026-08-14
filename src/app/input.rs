use smithay_client_toolkit::{
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{AxisScroll, BTN_LEFT, CursorIcon, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::WaylandSurface,
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_keyboard, wl_pointer, wl_surface},
};

use crate::{app::AppState, layout::LauncherLayout, model::Msg, style};

pub(super) const KEY_REPEAT_STEPS: usize = 1;
const MAX_WHEEL_STEPS_PER_EVENT: usize = 6;
const PIXELS_PER_WHEEL_STEP: f64 = 48.0;

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

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, surface: &wl_surface::WlSurface, _: u32) {
        if self.layer.wl_surface() == surface {
            self.keyboard_focus = false;
        }
    }

    fn press_key(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        match event.keysym {
            Keysym::Up => {
                self.dispatch(qh, Msg::SelectPrev);
            }
            Keysym::Down => {
                self.dispatch(qh, Msg::SelectNext);
            }
            _ => {
                if let Some(msg) = key_event_to_msg(&event) {
                    self.dispatch(qh, msg);
                }
            }
        }
    }

    fn repeat_key(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        match event.keysym {
            Keysym::Up => {
                for _ in 0..KEY_REPEAT_STEPS {
                    self.dispatch(qh, Msg::SelectPrev);
                }
            }
            Keysym::Down => {
                for _ in 0..KEY_REPEAT_STEPS {
                    self.dispatch(qh, Msg::SelectNext);
                }
            }
            _ => {
                if let Some(msg) = key_event_to_msg(&event) {
                    self.dispatch(qh, msg);
                }
            }
        }
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
    fn pointer_frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, _pointer: &wl_pointer::WlPointer, events: &[PointerEvent]) {
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
                PointerEventKind::Axis { vertical, .. } => {
                    for msg in wheel_messages(vertical) {
                        self.dispatch(qh, msg);
                    }
                }
                _ => {}
            }
        }
    }
}

impl AppState {
    fn update_cursor_for_position(&mut self, conn: &Connection, x: f64, y: f64) {
        let layout = self.model.layout();

        let over_clear = !self.model.launcher.query().is_empty() && layout.search_clear_rect().inset(-4).contains(x, y);

        let icon = if over_clear {
            CursorIcon::Pointer
        } else if layout.search.contains(x, y) {
            CursorIcon::Text
        } else if self.pointer_over_clickable(&layout, x, y) {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        };

        self.set_cursor_icon(conn, icon);
    }

    /// Manito sobre lo lanzable: tiles fijados y filas de apps (no separadores).
    fn pointer_over_clickable(&self, layout: &LauncherLayout, x: f64, y: f64) -> bool {
        let launcher = &self.model.launcher;

        if layout.pin_at(x, y, launcher.pinned_count(style::pins::MAX)).is_some() {
            return true;
        }

        let window_size = layout.visible_rows();
        let row_count = launcher.window_len(window_size);

        layout
            .row_at(x, y, row_count)
            .and_then(|row| launcher.entry_index_at_row(row, window_size))
            .is_some()
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

fn wheel_messages(vertical: AxisScroll) -> Vec<Msg> {
    let Some(direction) = wheel_direction(vertical) else {
        return Vec::new();
    };

    let steps = wheel_steps(vertical);

    (0..steps)
        .map(|_| if direction > 0 { Msg::SelectNext } else { Msg::SelectPrev })
        .collect()
}

fn wheel_direction(vertical: AxisScroll) -> Option<i32> {
    if vertical.value120 != 0 {
        return Some(vertical.value120.signum());
    }

    if vertical.discrete != 0 {
        return Some(vertical.discrete.signum());
    }

    if vertical.absolute > 0.0 {
        return Some(1);
    }

    if vertical.absolute < 0.0 {
        return Some(-1);
    }

    None
}

fn wheel_steps(vertical: AxisScroll) -> usize {
    let steps = if vertical.value120 != 0 {
        (vertical.value120.abs() / 120).max(1) as usize
    } else if vertical.discrete != 0 {
        vertical.discrete.unsigned_abs().max(1) as usize
    } else {
        (vertical.absolute.abs() / PIXELS_PER_WHEEL_STEP).round().max(1.0) as usize
    };

    steps.min(MAX_WHEEL_STEPS_PER_EVENT)
}
