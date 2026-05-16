use crate::{desktop::DesktopEntry, launcher::Launcher, layout::LauncherLayout};

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
            error: None,
        }
    }

    fn layout(&self) -> LauncherLayout {
        LauncherLayout::new(self.logical_width, self.logical_height, self.preferred_width, self.preferred_height)
    }
}

pub fn update(model: &mut Model, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::Type(ch) => redraw_if(model.launcher.push_char(ch)),
        Msg::Backspace => redraw_if(model.launcher.backspace()),
        Msg::SelectNext => redraw_if(model.launcher.select_next()),
        Msg::SelectPrev => redraw_if(model.launcher.select_prev()),
        Msg::HoverAt { x, y } => {
            let hovered = model.layout().row_at(x, y, model.launcher.visible_count());
            redraw_if(model.launcher.hover_index(hovered))
        }
        Msg::ClearHover => redraw_if(model.launcher.hover_index(None)),
        Msg::PointerPressedAt { x, y } => {
            let Some(index) = model.layout().row_at(x, y, model.launcher.visible_count()) else {
                return vec![];
            };
            model.launcher.select_index(index);
            launch_selected(model)
        }
        Msg::LaunchSelected => launch_selected(model),
        Msg::Quit => vec![Cmd::Exit],
        Msg::Configured { width, height } => {
            let size_changed = model.logical_width != width || model.logical_height != height;
            let first_configure = !model.configured;
            model.logical_width = width;
            model.logical_height = height;
            model.configured = true;
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
