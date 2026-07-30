mod input;
mod lifecycle;
mod renderer;

use anyhow::{Context, Result, anyhow, bail};
use calloop::timer::{TimeoutAction, Timer};
use calloop::{EventLoop, LoopHandle};
use calloop_wayland_source::WaylandSource;
use fontdue::Font;
use image::RgbaImage;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::{SeatState, pointer::ThemedPointer},
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
    },
    shm::{Shm, slot::SlotPool},
};
use std::time::Duration;
use wayland_client::{Connection, QueueHandle, globals::registry_queue_init, protocol::wl_keyboard};

use crate::{
    config::Config,
    desktop::{IconCache, launch_entry, scan_desktop_entries},
    font::load_ui_font,
    launcher::Launcher,
    model::{Cmd, Model, Msg, update},
    theme::Theme,
    usage::UsageStats,
    wallpaper_preview,
};

const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(530);

pub struct AppState {
    pub(super) registry_state: RegistryState,
    pub(super) seat_state: SeatState,
    pub(super) output_state: OutputState,
    pub(super) compositor: CompositorState,
    _layer_shell: LayerShell,
    pub(super) shm: Shm,
    pool: SlotPool,
    pub(super) layer: LayerSurface,
    pub(super) loop_handle: LoopHandle<'static, AppState>,
    qh: QueueHandle<AppState>,

    pub(super) redraw_scheduled: bool,
    has_rendered: bool,
    pub(super) should_close: bool,

    pub(super) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(super) keyboard_focus: bool,
    pub(super) themed_pointer: Option<ThemedPointer>,

    pub(super) model: Model,
    font: Font,
    theme: Theme,
    wallpaper_preview: Option<RgbaImage>,
    icon_cache: IconCache,
}

impl AppState {
    pub fn run(config: Config) -> Result<()> {
        let entries = scan_desktop_entries().context("no se pudieron leer aplicaciones .desktop")?;

        if entries.is_empty() {
            bail!("no encontré aplicaciones .desktop para mostrar");
        }

        let theme = Theme::load(&config);
        let wallpaper_preview = wallpaper_preview::load(theme.wallpaper.as_deref());

        let launcher = Launcher::new(entries, config.max_results, UsageStats::load());
        let model = Model::new(launcher, config.width, config.height);

        let mut icon_cache = IconCache::new();
        let visible_rows = model.layout().visible_rows();

        icon_cache.preload_entries(model.launcher.window_entries(visible_rows).into_iter().map(|(_, entry)| entry));

        icon_cache.preload_entries(model.launcher.visible_entries());

        let font = load_ui_font()?;

        let conn = Connection::connect_to_env().context("no se pudo conectar a Wayland")?;
        let (globals, event_queue) = registry_queue_init::<AppState>(&conn).context("registry_queue_init failed")?;
        let qh = event_queue.handle();

        let compositor = CompositorState::bind(&globals, &qh).context("wl_compositor no disponible")?;

        let layer_shell = LayerShell::bind(&globals, &qh).context("zwlr_layer_shell_v1 no disponible; Hyprland debería soportarlo")?;

        let shm = Shm::bind(&globals, &qh).context("wl_shm no disponible")?;

        let surface = compositor.create_surface(&qh);
        let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("hyprlauncher"), None);

        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer.set_exclusive_zone(-1);
        layer.set_size(0, 0);
        layer.commit();

        let pool = SlotPool::new((config.width * config.height * 4) as usize, &shm).context("no se pudo crear wl_shm SlotPool")?;

        let mut event_loop: EventLoop<AppState> = EventLoop::try_new().context("no se pudo crear calloop EventLoop")?;
        let loop_handle = event_loop.handle();

        let mut app = Self {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            compositor,
            _layer_shell: layer_shell,
            shm,
            pool,
            layer,
            loop_handle: loop_handle.clone(),
            qh: qh.clone(),
            redraw_scheduled: false,
            has_rendered: false,
            should_close: false,
            keyboard: None,
            keyboard_focus: false,
            themed_pointer: None,
            model,
            font,
            theme,
            wallpaper_preview,
            icon_cache,
        };

        WaylandSource::new(conn, event_queue)
            .insert(loop_handle)
            .map_err(|err| anyhow!("WaylandSource insert failed: {err:?}"))?;

        // Blinking search caret: a repeating timer flips visibility.
        event_loop
            .handle()
            .insert_source(Timer::from_duration(CARET_BLINK_INTERVAL), |_deadline, _, state: &mut AppState| {
                state.caret_blink_tick();
                TimeoutAction::ToDuration(CARET_BLINK_INTERVAL)
            })
            .map_err(|err| anyhow!("caret timer insert failed: {err:?}"))?;

        while !app.model.configured {
            event_loop.dispatch(None, &mut app).context("dispatch esperando configure")?;
        }

        while !app.should_close {
            event_loop.dispatch(None, &mut app).context("event_loop dispatch")?;
        }

        Ok(())
    }

    fn caret_blink_tick(&mut self) {
        let qh = self.qh.clone();
        self.dispatch(&qh, Msg::CaretBlink);
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
                    self.render_now(qh);
                }

                None
            }
            Cmd::Launch(entry) => match launch_entry(&entry) {
                Ok(()) => {
                    if let Err(err) = self.model.launcher.record_launch(&entry.id) {
                        log::warn!("no se pudo guardar el uso: {err:#}");
                    }

                    None
                }
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
