mod input;
mod lifecycle;
mod renderer;

use anyhow::{Context, Result, bail};
use fontdue::Font;
use image::RgbaImage;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
    },
    shm::{Shm, slot::SlotPool},
};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_pointer},
};

use crate::{
    config::Config,
    desktop::{launch_entry, scan_desktop_entries},
    font::load_ui_font,
    launcher::Launcher,
    model::{Cmd, Model, Msg, update},
    theme::Theme,
    wallpaper_preview,
};

pub struct AppState {
    pub(super) registry_state: RegistryState,
    pub(super) seat_state: SeatState,
    pub(super) output_state: OutputState,
    _compositor: CompositorState,
    _layer_shell: LayerShell,
    pub(super) shm: Shm,
    pool: SlotPool,
    pub(super) layer: LayerSurface,

    pub(super) redraw_scheduled: bool,
    has_rendered: bool,
    pub(super) should_close: bool,

    pub(super) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(super) keyboard_focus: bool,
    pub(super) pointer: Option<wl_pointer::WlPointer>,

    pub(super) model: Model,
    font: Font,
    theme: Theme,
    wallpaper_preview: Option<RgbaImage>,
}

impl AppState {
    pub fn run(config: Config) -> Result<()> {
        let entries = scan_desktop_entries().context("no se pudieron leer aplicaciones .desktop")?;
        if entries.is_empty() {
            bail!("no encontré aplicaciones .desktop para mostrar");
        }

        let theme = Theme::load(&config);
        let wallpaper_preview = wallpaper_preview::load(theme.wallpaper.as_deref());
        let launcher = Launcher::new(entries, config.max_results);
        let model = Model::new(launcher, config.width, config.height);
        let font = load_ui_font()?;

        let conn = Connection::connect_to_env().context("no se pudo conectar a Wayland")?;
        let (globals, mut event_queue) =
            registry_queue_init::<AppState>(&conn).context("registry_queue_init failed")?;
        let qh = event_queue.handle();

        let compositor = CompositorState::bind(&globals, &qh).context("wl_compositor no disponible")?;
        let layer_shell = LayerShell::bind(&globals, &qh)
            .context("zwlr_layer_shell_v1 no disponible; Hyprland debería soportarlo")?;
        let shm = Shm::bind(&globals, &qh).context("wl_shm no disponible")?;

        let surface = compositor.create_surface(&qh);
        let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("hyprlauncher"), None);

        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer.set_exclusive_zone(-1);
        layer.set_size(0, 0);
        layer.commit();

        let pool = SlotPool::new((config.width * config.height * 4) as usize, &shm)
            .context("no se pudo crear wl_shm SlotPool")?;

        let mut app = Self {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            _compositor: compositor,
            _layer_shell: layer_shell,
            shm,
            pool,
            layer,
            redraw_scheduled: false,
            has_rendered: false,
            should_close: false,
            keyboard: None,
            keyboard_focus: false,
            pointer: None,
            model,
            font,
            theme,
            wallpaper_preview,
        };

        while !app.model.configured {
            event_queue
                .blocking_dispatch(&mut app)
                .context("dispatch esperando configure")?;
        }

        while !app.should_close {
            event_queue
                .blocking_dispatch(&mut app)
                .context("event_queue dispatch")?;
        }

        Ok(())
    }

    pub(super) fn dispatch(&mut self, qh: &QueueHandle<Self>, msg: Msg) {
        let mut pending = vec![msg];

        while let Some(msg) = pending.pop() {
            for cmd in update(&mut self.model, msg) {
                if let Some(followup) = self.execute(qh, cmd) {
                    pending.push(followup);
                }
            }
        }
    }

    fn execute(&mut self, qh: &QueueHandle<Self>, cmd: Cmd) -> Option<Msg> {
        match cmd {
            Cmd::Redraw => {
                if self.has_rendered {
                    self.request_redraw(qh);
                } else {
                    self.render_now();
                }
                None
            }
            Cmd::Launch(entry) => match launch_entry(&entry) {
                Ok(()) => None,
                Err(err) => Some(Msg::LaunchFailed(format!("{err:#}"))),
            },
            Cmd::SetBufferScale(scale) => {
                self.layer.wl_surface().set_buffer_scale(scale);
                None
            }
            Cmd::Exit => {
                self.should_close = true;
                None
            }
        }
    }
}
