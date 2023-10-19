mod clock;
mod segment;
mod tabs;

use clock::Clock;
use segment::*;
use tabs::Tabs;
use zellij_tile::prelude::*;

use std::{cell::RefCell, collections::BTreeMap, fmt::Display, ops::Deref, rc::Rc};

use anstyle::{Color, Style as AnsiStyle};
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
