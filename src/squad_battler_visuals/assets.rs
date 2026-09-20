use bevy::prelude::Color;

pub fn clear_color() -> Color {
    Color::rgb(0.055, 0.041, 0.03)
}

pub fn table_color() -> Color {
    Color::rgb(0.25, 0.14, 0.075)
}

pub fn board_edge_color() -> Color {
    Color::rgb(0.39, 0.22, 0.11)
}

pub fn tile_light_color() -> Color {
    Color::rgb(0.47, 0.31, 0.17)
}

pub fn tile_dark_color() -> Color {
    Color::rgb(0.36, 0.225, 0.125)
}

pub fn grid_line_color() -> Color {
    Color::rgba(0.13, 0.07, 0.035, 0.72)
}

pub fn player_outer_color() -> Color {
    Color::rgb(0.96, 0.75, 0.31)
}

pub fn player_inner_color() -> Color {
    Color::rgb(0.17, 0.24, 0.28)
}

pub fn enemy_outer_color() -> Color {
    Color::rgb(0.78, 0.24, 0.16)
}

pub fn enemy_inner_color() -> Color {
    Color::rgb(0.28, 0.105, 0.085)
}

pub fn downed_color() -> Color {
    Color::rgb(0.16, 0.13, 0.11)
}

pub fn health_back_color() -> Color {
    Color::rgb(0.07, 0.035, 0.025)
}

pub fn health_high_color() -> Color {
    Color::rgb(0.62, 0.78, 0.43)
}

pub fn health_mid_color() -> Color {
    Color::rgb(0.93, 0.68, 0.28)
}

pub fn health_low_color() -> Color {
    Color::rgb(0.74, 0.19, 0.13)
}
