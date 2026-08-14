use crate::{desktop::DesktopEntry, launcher::Launcher, layout::LauncherLayout, style};

#[derive(Debug, Clone)]
pub enum Msg {
    Type(char),
    Backspace,
    SelectNext,
    SelectPrev,
    HoverAt { x: f64, y: f64 },
    ClearHover,
    PointerPressedAt { x: f64, y: f64 },
    LaunchSelected,
    Quit,
    Configured { width: u32, height: u32 },
    ScaleChanged(i32),
    LaunchFailed(String),
    CaretBlink,
}

#[derive(Debug)]
pub enum Cmd {
    Redraw,
    Launch(DesktopEntry),
    SetBufferScale(i32),
    Exit,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub launcher: Launcher,
    pub scale: i32,
    pub logical_width: u32,
    pub logical_height: u32,
    pub preferred_width: u32,
    pub preferred_height: u32,
    pub configured: bool,
    pub search_focused: bool,
    pub caret_visible: bool,
    pub hovered_pin: Option<usize>,
    pub error: Option<String>,
}

impl Model {
    pub fn new(launcher: Launcher, preferred_width: u32, preferred_height: u32) -> Self {
        Self {
            launcher,
            scale: 1,
            logical_width: preferred_width,
            logical_height: preferred_height,
            preferred_width,
            preferred_height,
            configured: false,
            // El launcher captura el teclado apenas abre: el input arranca
            // enfocado para invitar a tipear.
            search_focused: true,
            caret_visible: true,
            hovered_pin: None,
            error: None,
        }
    }

    pub fn layout(&self) -> LauncherLayout {
        LauncherLayout::new(
            self.logical_width,
            self.logical_height,
            self.preferred_width,
            self.preferred_height,
            self.launcher.pinned_count(style::pins::MAX),
        )
    }

    fn visible_rows(&self) -> usize {
        self.layout().visible_rows()
    }
}

pub fn update(model: &mut Model, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::Type(ch) => {
            model.search_focused = true;
            model.caret_visible = true;
            let changed = model.launcher.push_char(ch);
            let scrolled = model.launcher.ensure_selected_visible(model.visible_rows());
            redraw_if(changed || scrolled)
        }
        Msg::Backspace => {
            model.search_focused = true;
            model.caret_visible = true;
            let changed = model.launcher.backspace();
            let scrolled = model.launcher.ensure_selected_visible(model.visible_rows());
            redraw_if(changed || scrolled)
        }
        Msg::SelectNext => {
            let changed = model.launcher.select_next();
            let scrolled = model.launcher.ensure_selected_visible(model.visible_rows());
            redraw_if(changed || scrolled)
        }
        Msg::SelectPrev => {
            let changed = model.launcher.select_prev();
            let scrolled = model.launcher.ensure_selected_visible(model.visible_rows());
            redraw_if(changed || scrolled)
        }
        Msg::HoverAt { x, y } => {
            let layout = model.layout();
            let window_size = layout.visible_rows();
            let row_count = model.launcher.window_len(window_size);

            let hovered = layout
                .row_at(x, y, row_count)
                .and_then(|row| model.launcher.entry_index_at_row(row, window_size));

            let pin_hovered = layout.pin_at(x, y, model.launcher.pinned_count(style::pins::MAX));
            let pins_changed = model.hovered_pin != pin_hovered;
            model.hovered_pin = pin_hovered;

            redraw_if(model.launcher.hover_index(hovered) || pins_changed)
        }
        Msg::ClearHover => {
            let pins_changed = model.hovered_pin.is_some();
            model.hovered_pin = None;

            let rows_changed = model.launcher.hover_index(None);

            redraw_if(rows_changed || pins_changed)
        }
        Msg::PointerPressedAt { x, y } => {
            let layout = model.layout();

            if layout.search.contains(x, y) {
                // La × limpia la búsqueda (con área de click generosa).
                if !model.launcher.query().is_empty() && layout.search_clear_rect().inset(-4).contains(x, y) {
                    model.launcher.clear_query();
                    model.search_focused = true;
                    model.caret_visible = true;
                    return vec![Cmd::Redraw];
                }

                let changed = !model.search_focused;
                model.search_focused = true;
                return redraw_if(changed);
            }

            if let Some(entry) = layout
                .pin_at(x, y, model.launcher.pinned_count(style::pins::MAX))
                .and_then(|index| model.launcher.pinned_entry(index, style::pins::MAX))
            {
                model.search_focused = false;
                return vec![Cmd::Launch(entry), Cmd::Exit];
            }

            let window_size = layout.visible_rows();
            let row_count = model.launcher.window_len(window_size);

            if let Some(index) = layout
                .row_at(x, y, row_count)
                .and_then(|row| model.launcher.entry_index_at_row(row, window_size))
            {
                model.search_focused = false;
                model.launcher.select_index(index);
                return launch_selected(model);
            }

            let changed = model.search_focused;
            model.search_focused = false;
            redraw_if(changed)
        }
        Msg::LaunchSelected => launch_selected(model),
        Msg::Quit => vec![Cmd::Exit],
        Msg::Configured { width, height } => {
            let size_changed = model.logical_width != width || model.logical_height != height;
            let first_configure = !model.configured;

            model.logical_width = width;
            model.logical_height = height;
            model.configured = true;
            model.launcher.ensure_selected_visible(model.visible_rows());

            if size_changed || first_configure {
                vec![Cmd::Redraw]
            } else {
                vec![]
            }
        }
        Msg::ScaleChanged(new_scale) => {
            if new_scale < 1 || new_scale == model.scale {
                return vec![];
            }

            model.scale = new_scale;

            let mut cmds = vec![Cmd::SetBufferScale(new_scale)];

            if model.configured {
                cmds.push(Cmd::Redraw);
            }

            cmds
        }
        Msg::LaunchFailed(error) => {
            model.error = Some(error);
            vec![Cmd::Redraw]
        }
        Msg::CaretBlink => {
            if !model.search_focused {
                model.caret_visible = true;
                return vec![];
            }
            model.caret_visible = !model.caret_visible;
            vec![Cmd::Redraw]
        }
    }
}

fn launch_selected(model: &mut Model) -> Vec<Cmd> {
    match model.launcher.selected_entry() {
        Some(entry) => vec![Cmd::Launch(entry), Cmd::Exit],
        None => vec![],
    }
}

fn redraw_if(changed: bool) -> Vec<Cmd> {
    if changed { vec![Cmd::Redraw] } else { vec![] }
}
