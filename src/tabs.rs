use std::fmt::Display;

use zellij_tile::prelude::TabInfo;

use crate::{DisplayExt, Segment};

#[derive(Default)]
pub struct Tabs {
    pub max_width: usize,

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
    pub fn new(inner: Vec<TabInfo>) -> Self {
        let full: String = inner
            .iter()
            .map(Segment::new_tab)
            .map(|x| x.to_string())
            .collect();

        let compact: String = inner
            .iter()
            .map(Segment::new_compact_tab)
            .map(|x| x.to_string())
            .collect();

        let last = inner.len() - 1;
        let fold = if let Some(active) = inner.iter().find(|x| x.active) {
            let mut active_segment = Segment::new_tab(active).to_string();
            if active.position != 0 {
                active_segment = format!(
                    "{}{active_segment}",
                    Segment::new_range_tab(0..active.position - 1)
                );
            }
            if active.position != last {
                active_segment = format!(
                    "{active_segment}{}",
                    Segment::new_range_tab(active.position + 1..last)
                )
            }
            active_segment.to_string()
        } else {
            Segment::new_range_tab(0..last).to_string()
        };

        Self {
            max_width: usize::MAX,

            full: (full.display_len(), full),
            compact: (compact.display_len(), compact),
            fold: (fold.display_len(), fold),
        }
    }
}
