use std::{fmt::Display, ops::Range};

use anstyle::{AnsiColor, Color, RgbColor, Style as AnsiStyle};
use lazy_static::lazy_static;
use unicode_segmentation::UnicodeSegmentation;
use zellij_tile::prelude::*;

lazy_static! {
    pub static ref BG: Option<Color> = Some(AnsiColor::Black.into());
    pub static ref RED: Option<Color> = Some(AnsiColor::Red.into());
    pub static ref GREEN: Option<Color> = Some(AnsiColor::Green.into());
    pub static ref YELLOW: Option<Color> = Some(AnsiColor::Yellow.into());
    pub static ref BLUE: Option<Color> = Some(AnsiColor::Blue.into());
    pub static ref MAGENTA: Option<Color> = Some(AnsiColor::Magenta.into());
    pub static ref CYAN: Option<Color> = Some(AnsiColor::Cyan.into());
    pub static ref GRAY: Option<Color> = Some(AnsiColor::White.into());
    pub static ref WHITE: Option<Color> = Some(AnsiColor::BrightWhite.into());
    pub static ref BLACK: Option<Color> = Some(RgbColor(0u8, 0u8, 0u8).into());
}

pub struct Segment {
    content: Box<dyn Display>,
    style: AnsiStyle,

    min_content_width: usize,
    max_content_width: usize,
    padding_left: &'static str,
    padding_right: &'static str,
    begin: &'static str,
    end: &'static str,
}

impl Segment {
    pub fn new(content: Box<dyn Display>, style: AnsiStyle) -> Self {
        Segment {
            content,
            style,

            ..Default::default()
        }
    }

    pub fn new_tab(tab: &TabInfo) -> Self {
        let color = if tab.active { *YELLOW } else { *GRAY };
        let fullscreen = if tab.is_fullscreen_active { "󰊓" } else { "" };
        let sync = if tab.is_sync_panes_active { "󱍸" } else { "" };
        let content = format!(
            "{}  {} {}{}",
            tab.position + 1,
            tab.name.clone(),
            sync,
            fullscreen
        );
        Segment {
            content: Box::new(content),
            style: AnsiStyle::new().fg_color(*BLACK).bg_color(color),

            ..Default::default()
        }
    }

    pub fn new_compact_tab(tab: &TabInfo) -> Self {
        if tab.active {
            return Self::new_tab(tab);
        }

        let color = if tab.active { *YELLOW } else { *GRAY };
        let content = format!("{}", tab.position + 1);
        Segment {
            content: Box::new(content),
            style: AnsiStyle::new().fg_color(*BLACK).bg_color(color),

            ..Default::default()
        }
    }

    pub fn new_range_tab(range: Range<usize>) -> Self {
        let content = if range.is_empty() {
            format!("{}", range.start + 1)
        } else {
            format!("{}  󰜴  {}", range.start + 1, range.end + 1)
        };
        Segment {
            content: Box::new(content),
            style: AnsiStyle::new().fg_color(*BLACK).bg_color(*GRAY),

            ..Default::default()
        }
    }

    pub fn min_width(mut self, width: usize) -> Self {
        self.min_content_width = width;
        self
    }

    pub fn max_width(mut self, width: usize) -> Self {
        self.max_content_width = width;
        self
    }
}

impl Default for Segment {
    fn default() -> Self {
        Segment {
            content: Box::new(""),
            style: AnsiStyle::new().fg_color(*GRAY).bg_color(*BG),

            min_content_width: 0,
            max_content_width: 32,

            padding_left: " ",
            padding_right: " ",

            begin: "",
            end: "",
        }
    }
}

impl Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut content = self.content.to_string();
        if content.graphemes(true).count() > self.max_content_width {
            content = format!(
                "{}...",
                content
                    .graphemes(true)
                    .take(self.max_content_width - 3)
                    .collect::<String>()
            );
        }

        let begin_style = AnsiStyle::new()
            .bg_color(self.style.get_bg_color())
            .fg_color(*BG)
            .render();
        let end_style = AnsiStyle::new()
            .fg_color(self.style.get_bg_color())
            .bg_color(*BG)
            .render();
        let reset = self.style.render_reset();
        let style = self.style.render();

        write!(
            f,
            "{begin_style}{begin}{style}{padding_left}{content:^width$}{padding_right}{reset}{end_style}{end}",
            width = self.min_content_width,
            padding_left = self.padding_left,
            padding_right = self.padding_right,
            begin = self.begin,
            end = self.end,
        )
    }
}
