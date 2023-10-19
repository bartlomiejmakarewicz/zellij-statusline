use zellij_tile::prelude::*;

use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt::Display,
    ops::{Deref, Range},
    rc::Rc,
};

use anstyle::{AnsiColor, Color, RgbColor, Style as AnsiStyle};
use chrono_tz::Tz;
use lazy_static::lazy_static;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
pub struct PluginState {
    pub config: BTreeMap<String, String>,

    pub mode: Shared<String>,
    pub session: Shared<String>,
    pub tabs: Tabs,

    pub left_elements: Vec<Box<dyn Display>>,
    pub right_elements: Vec<Box<dyn Display>>,
}

// NOTE: Plugin configuration keys
const TZ_STRING: &str = "timezone";
const SELECTABLE: &str = "selectable";

// NOTE: Plugin has opinionated approach to the theme, and inherits most color from term
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

register_plugin!(PluginState);

impl ZellijPlugin for PluginState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[PermissionType::ReadApplicationState]);
        subscribe(&[
            EventType::ModeUpdate,
            EventType::SessionUpdate,
            EventType::TabUpdate,
        ]);

        self.config = configuration;
        self.session = Shared::new("default".into());

        if let Some(value) = self.config.get(SELECTABLE) {
            let selectable: bool = value.parse().unwrap_or(false);
            set_selectable(selectable);
        }

        // NOTE: create segments
        // INFO: MODE
        // Not internally mutable without `update` call - we can render it to String
        let mode = Mode(InputMode::Normal);
        let segment = Segment::new(
            Box::new(mode),
            AnsiStyle::new()
                .bg_color(mode.color())
                .fg_color(*BLACK)
                .bold(),
        )
        .min_width(10);
        *self.mode.borrow_mut() = segment.to_string();
        self.left_elements.push(Box::new(self.mode.clone()));

        // INFO: SESSION
        // Not internally mutable without `update` call - we can render it to String
        let segment = Segment::new(
            Box::new("default"),
            AnsiStyle::new().fg_color(*BLACK).bg_color(*GREEN),
        )
        .min_width(10);
        self.left_elements.push(Box::new(segment.to_string()));

        // INFO: CLOCK
        // This segment actually change its display, so we are not prerendering it
        let segment = Segment::new(
            Box::new(Clock::new(self.config.get(TZ_STRING))),
            AnsiStyle::new().bg_color(*WHITE).fg_color(*BLACK),
        )
        .max_width(64);
        self.right_elements.push(Box::new(segment));
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::ModeUpdate(event) => {
                let mode = Mode(event.mode);
                let segment = Segment::new(
                    Box::new(mode),
                    AnsiStyle::new()
                        .bg_color(mode.color())
                        .fg_color(*BLACK)
                        .bold(),
                )
                .min_width(10);

                // INFO: render updated state to String
                *self.mode.borrow_mut() = segment.to_string();
                should_render = true;
            }
            Event::SessionUpdate(sessions) => {
                for session in sessions {
                    if session.is_current_session {
                        let segment = Segment::new(
                            Box::new(session.name),
                            AnsiStyle::new().bg_color(*GREEN).fg_color(*BLACK),
                        )
                        .min_width(10);
                        //
                        // INFO: render updated state to String
                        *self.session.borrow_mut() = segment.to_string();

                        should_render = true;
                        break;
                    }
                }
            }
            Event::TabUpdate(tabs) => {
                self.tabs = Tabs::new(tabs);
                should_render = true;
            }
            _ => {}
        }

        should_render
    }

    fn render(&mut self, _: usize, cols: usize) {
        let mut chars = 0;

        // NOTE: render left segments
        for s in &self.left_elements {
            chars += s.display_len();
            print!("{s}");
        }

        // NOTE: eat right segments chars before rendering to let TABS know how much space they have left
        for s in &self.right_elements {
            chars += s.display_len();
        }

        // NOTE: render tabs
        self.tabs.max_width = cols - chars;
        chars += self.tabs.display_len();
        print!("{}", self.tabs);

        // NOTE: fill empty space
        if chars < cols {
            let fill = "-".to_string().repeat(cols - chars);
            print!(
                "{}{}",
                AnsiStyle::new().fg_color(*GRAY).bg_color(*BG).render(),
                fill
            );
        }

        // NOTE: render right segments
        for s in &self.right_elements {
            print!("{s}");
        }
    }
}

trait DisplayExt {
    fn display_len(&self) -> usize;
}

impl<T: Display> DisplayExt for T {
    fn display_len(&self) -> usize {
        strip_ansi_escapes::strip_str(self.to_string())
            .graphemes(true)
            .count()
    }
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
    fn new(content: Box<dyn Display>, style: AnsiStyle) -> Self {
        Segment {
            content,
            style,

            ..Default::default()
        }
    }

    fn new_tab(tab: &TabInfo) -> Self {
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

    fn new_compact_tab(tab: &TabInfo) -> Self {
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

    fn new_range_tab(range: Range<usize>) -> Self {
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

#[derive(Default)]
pub struct Segments(Vec<Segment>);

impl Display for Segments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in &self.0 {
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub struct Mode(InputMode);

impl PartialEq<InputMode> for Mode {
    fn eq(&self, other: &InputMode) -> bool {
        self.0.eq(other)
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self.0 {
            InputMode::Normal => "NORMAL",
            InputMode::Locked => "LOCKED",
            InputMode::Resize => "RESIZE",
            InputMode::Pane => "PANE",
            InputMode::Tab => "TAB",
            InputMode::Scroll => "SCROLL",
            InputMode::EnterSearch => "SEARCH",
            InputMode::Search => "SEARCH",
            InputMode::RenameTab => "TAB",
            InputMode::RenamePane => "PANE",
            InputMode::Session => "SESSION",
            InputMode::Move => "MOVE",
            InputMode::Prompt => "PROMPT",
            InputMode::Tmux => "TMUX",
        };
        write!(f, "{repr}")
    }
}

impl Mode {
    fn color(&self) -> Option<Color> {
        match self.0 {
            InputMode::Normal => *BLUE,
            InputMode::Locked => *GRAY,
            InputMode::Tmux => *RED,
            InputMode::Scroll | InputMode::EnterSearch | InputMode::Search => *MAGENTA,
            _ => *YELLOW,
        }
    }
}

#[derive(Default)]
pub struct Tabs {
    max_width: usize,

    full: (usize, String),
    compact: (usize, String),
    fold: (usize, String),
}

impl Display for Tabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = if self.max_width > self.full.0 {
            &self.full.1
        } else if self.max_width > self.compact.0 {
            &self.compact.1
        } else {
            &self.fold.1
        };
        write!(f, "{content}")
    }
}

impl Tabs {
    fn new(inner: Vec<TabInfo>) -> Self {
        let full = Segments(inner.iter().map(Segment::new_tab).collect());
        let compact = Segments(inner.iter().map(Segment::new_compact_tab).collect());
        let last = inner.len() - 1;
        let fold = if let Some(active) = inner.iter().find(|x| x.active) {
            let active_segment = Segment::new_tab(active);
            if active.position == 0 {
                Segments(vec![active_segment, Segment::new_range_tab(1..last)])
            } else if active.position == last {
                Segments(vec![Segment::new_range_tab(0..last - 1), active_segment])
            } else {
                Segments(vec![
                    Segment::new_range_tab(0..active.position - 1),
                    active_segment,
                    Segment::new_range_tab(active.position + 1..last),
                ])
            }
        } else {
            Segments(vec![Segment::new_range_tab(0..last)])
        };

        let full = full.to_string();
        let compact = compact.to_string();
        let fold = fold.to_string();

        Self {
            max_width: usize::MAX,

            full: (full.display_len(), full),
            compact: (compact.display_len(), compact),
            fold: (fold.display_len(), fold),
        }
    }
}

#[derive(Default)]
pub struct DisplayRefCell<T>(RefCell<T>);

impl<T> Deref for DisplayRefCell<T> {
    type Target = RefCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Display for DisplayRefCell<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

impl<T> DisplayRefCell<T> {
    fn new(inner: T) -> Self {
        Self(RefCell::new(inner))
    }
}

#[derive(Default)]
pub struct Shared<T>(Rc<DisplayRefCell<T>>);

impl<T> Deref for Shared<T> {
    type Target = Rc<DisplayRefCell<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Display for Shared<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

impl<T> Shared<T> {
    fn new(inner: T) -> Self {
        Self(Rc::new(DisplayRefCell::new(inner)))
    }
}

pub struct Clock {
    tz: Tz,
    format: String,
}

impl Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let now = chrono::Local::now();
        write!(f, "{}", now.with_timezone(&self.tz).format(&self.format))
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            tz: Tz::UTC,
            format: "󰅐 %Y-%m-%dT%H:%M:%S%:z  epoch: %s".to_string(),
        }
    }
}

impl Clock {
    fn new(tz: Option<&String>) -> Self {
        let mut clock = Self::default();
        if let Some(tz) = tz {
            if let Ok(tz) = tz.parse() {
                clock.tz = tz;
            }
        };
        clock
    }
}
