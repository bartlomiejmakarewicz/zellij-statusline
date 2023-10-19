use chrono_tz::Tz;

use std::fmt::Display;

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
    pub fn new(tz: Option<&String>) -> Self {
        let mut clock = Self::default();
        if let Some(tz) = tz {
            if let Ok(tz) = tz.parse() {
                clock.tz = tz;
            }
        };
        clock
    }
}
