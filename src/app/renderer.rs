use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::{QueueHandle, protocol::wl_shm};

use crate::render::{RenderRequest, render_launcher};

use super::AppState;

impl AppState {
    pub(super) fn request_redraw(&mut self, qh: &QueueHandle<AppState>) {
        if self.redraw_scheduled {
            return;
        }

        self.redraw_scheduled = true;

        let wl_surface = self.layer.wl_surface().clone();
        wl_surface.frame(qh, wl_surface.clone());
        self.layer.commit();
    }

    pub(super) fn render_now(&mut self, qh: &QueueHandle<AppState>) {
        let logical_w = self.model.logical_width.max(1);
        let logical_h = self.model.logical_height.max(1);

        let scale = self.model.scale.max(1) as u32;
        let phys_w = logical_w * scale;
        let phys_h = logical_h * scale;
        let stride = phys_w as i32 * 4;

        let wl_surface = self.layer.wl_surface().clone();

        let Ok((buffer, canvas)) = self
            .pool
            .create_buffer(phys_w as i32, phys_h as i32, stride, wl_shm::Format::Argb8888)
        else {
            log::error!("no se pudo crear buffer SHM");
            return;
        };

        canvas.fill(0);

        render_launcher(RenderRequest {
            canvas,
            width: phys_w,
            height: phys_h,
            scale: scale as f32,
            model: &self.model,
            theme: &self.theme,
            wallpaper: self.wallpaper_preview.as_ref(),
            icons: &mut self.icon_cache,
            font: &self.font,
        });

        wl_surface.damage_buffer(0, 0, phys_w as i32, phys_h as i32);

        if let Err(err) = buffer.attach_to(&wl_surface) {
            log::error!("buffer attach failed: {err:?}");
            return;
        }

        self.layer.commit();
        self.has_rendered = true;

        if self.icon_cache.needs_redraw() {
            self.request_redraw(qh);
        }
    }
}
