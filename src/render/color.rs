//! Color-mode handling (`auto`/`always`/`never`) and `NO_COLOR` support.

use std::io::IsTerminal;

/// When the renderer should emit ANSI color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Color only when writing to a terminal and `NO_COLOR` is unset.
    #[default]
    Auto,
    /// Always color, even when piped (overrides `NO_COLOR`).
    Always,
    /// Never color.
    Never,
}

impl ColorMode {
    /// Whether color should be emitted to stdout under this mode.
    ///
    /// `Auto` respects the [`NO_COLOR`](https://no-color.org) convention and
    /// only colors an interactive terminal. An explicit `Always` overrides
    /// `NO_COLOR`, matching common CLI behavior.
    pub fn colorize(self) -> bool {
        match self {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => no_color_unset() && std::io::stdout().is_terminal(),
        }
    }
}

/// True when the `NO_COLOR` environment variable is not set (to any value).
fn no_color_unset() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}
