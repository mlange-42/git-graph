//! ANSI terminal color handling.

use lazy_static::lazy_static;
use std::collections::HashMap;

/// Converts a color name to the index in the 256-color palette.
pub fn to_terminal_color(color: &str) -> Result<u8, String> {
    match NAMED_COLORS.get(color) {
        None => match color.parse::<u8>() {
            Ok(col) => Ok(col),
            Err(_) => Err(format!("Color {} not found", color)),
        },
        Some(rgb) => Ok(*rgb),
    }
}

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

lazy_static! {
    /// Named ANSI colors
    pub static ref NAMED_COLORS: HashMap<&'static str, u8> = hashmap![
        "black" => 0,
        "red" => 1,
        "green" => 2,
        "yellow" => 3,
        "blue" => 4,
        "magenta" => 5,
        "cyan" => 6,
        "white" => 7,
        "bright_black" => 8,
        "bright_red" => 9,
        "bright_green" => 10,
        "bright_yellow" => 11,
        "bright_blue" => 12,
        "bright_magenta" => 13,
        "bright_cyan" => 14,
        "bright_white" => 15
    ];
}
