pub mod surface {
    pub const WIDTH_HINT: u32 = 760;
    pub const HEIGHT_HINT: u32 = 420;
    pub const PANEL_RADIUS: i32 = 22;
    /// El preview va a sangre contra el panel (dejando 1px de borde visible),
    /// así que su radio acompaña al del panel.
    pub const PREVIEW_RADIUS: i32 = PANEL_RADIUS - 1;
    pub const ITEM_RADIUS: i32 = 16;
    /// Mismo radio que los tiles fijados: un solo lenguaje de formas.
    pub const SEARCH_RADIUS: i32 = 12;
}

pub mod spacing {
    pub const PANEL_PADDING: i32 = 18;
    pub const GAP: i32 = 18;
    pub const SEARCH_HEIGHT: i32 = 40;
    pub const SEARCH_ICON_SIZE: i32 = 18;
    pub const ROW_HEIGHT: i32 = 44;
    pub const ROW_GAP: i32 = 8;
    pub const ICON_SIZE: i32 = 26;
}

/// Contenedor de apps fijadas (las más usadas) entre la búsqueda y la lista.
/// Tiles solo con el ícono, sin nombre.
pub mod pins {
    pub const MAX: usize = 5;
    pub const LABEL_HEIGHT: i32 = 14;
    pub const LABEL_GAP: i32 = 6;
    pub const TILE_HEIGHT: i32 = 40;
    pub const GAP: i32 = 8;
    pub const RADIUS: i32 = 12;
    pub const ICON_SIZE: i32 = 24;
}

pub mod font_size {
    pub const TITLE: f32 = 15.5;
    pub const QUERY: f32 = 14.0;
    pub const HINT: f32 = 11.5;
}
