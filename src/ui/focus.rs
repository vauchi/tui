// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Focus Management System
//!
//! Handles keyboard navigation across three zones:
//! - Content: UP/DOWN cycles focusable widgets
//! - ActionBar: LEFT/RIGHT cycles actions
//! - NavBar: LEFT/RIGHT cycles tabs
//!
//! Wrapping: RIGHT at action bar end → nav bar start;
//! LEFT at nav bar start → action bar end.

/// The zone currently holding focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusZone {
    Content,
    ActionBar,
    NavBar,
}

/// Manages focus state across Content, ActionBar, and NavBar zones.
#[derive(Clone, Debug)]
pub struct FocusManager {
    pub zone: FocusZone,
    pub content_index: usize,
    pub action_index: usize,
    pub nav_index: usize,
    content_count: usize,
    action_count: usize,
    nav_count: usize,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            zone: FocusZone::Content,
            content_index: 0,
            action_index: 0,
            nav_index: 0, // Home (My Card tab)
            content_count: 0,
            action_count: 0,
            nav_count: 5,
        }
    }

    /// Update the number of focusable items per zone.
    pub fn set_counts(&mut self, content: usize, action: usize, nav: usize) {
        self.content_count = content;
        self.action_count = action;
        self.nav_count = nav;
        // Clamp indices
        if self.content_count > 0 && self.content_index >= self.content_count {
            self.content_index = self.content_count - 1;
        }
        if self.action_count > 0 && self.action_index >= self.action_count {
            self.action_index = self.action_count - 1;
        }
        if self.nav_count > 0 && self.nav_index >= self.nav_count {
            self.nav_index = self.nav_count - 1;
        }
    }

    /// Move focus up. In Content zone, cycles upward. At top of content, no-op.
    pub fn move_up(&mut self) {
        match self.zone {
            FocusZone::Content => {
                if self.content_count > 0 && self.content_index > 0 {
                    self.content_index -= 1;
                }
            }
            FocusZone::ActionBar | FocusZone::NavBar => {
                // Move from bars back to content
                if self.content_count > 0 {
                    self.zone = FocusZone::Content;
                }
            }
        }
    }

    /// Move focus down. In Content, cycles down. Past content bottom → ActionBar.
    pub fn move_down(&mut self) {
        match self.zone {
            FocusZone::Content => {
                if self.content_count > 0 && self.content_index < self.content_count - 1 {
                    self.content_index += 1;
                } else if self.action_count > 0 {
                    self.zone = FocusZone::ActionBar;
                } else if self.nav_count > 0 {
                    self.zone = FocusZone::NavBar;
                }
            }
            FocusZone::ActionBar => {
                if self.nav_count > 0 {
                    self.zone = FocusZone::NavBar;
                }
            }
            FocusZone::NavBar => {
                // At bottom, no-op
            }
        }
    }

    /// Move focus left. In bars, cycles left with wrapping between bars.
    pub fn move_left(&mut self) {
        match self.zone {
            FocusZone::Content => {}
            FocusZone::ActionBar => {
                if self.action_index > 0 {
                    self.action_index -= 1;
                } else if self.nav_count > 0 {
                    // Wrap to nav bar end
                    self.zone = FocusZone::NavBar;
                    self.nav_index = self.nav_count.saturating_sub(1);
                }
            }
            FocusZone::NavBar => {
                if self.nav_index > 0 {
                    self.nav_index -= 1;
                } else if self.action_count > 0 {
                    // Wrap to action bar end
                    self.zone = FocusZone::ActionBar;
                    self.action_index = self.action_count.saturating_sub(1);
                }
            }
        }
    }

    /// Move focus right. In bars, cycles right with wrapping between bars.
    pub fn move_right(&mut self) {
        match self.zone {
            FocusZone::Content => {}
            FocusZone::ActionBar => {
                if self.action_count > 0 && self.action_index < self.action_count - 1 {
                    self.action_index += 1;
                } else if self.nav_count > 0 {
                    // Wrap to nav bar start
                    self.zone = FocusZone::NavBar;
                    self.nav_index = 0;
                }
            }
            FocusZone::NavBar => {
                if self.nav_count > 0 && self.nav_index < self.nav_count - 1 {
                    self.nav_index += 1;
                } else if self.action_count > 0 {
                    // Wrap to action bar start
                    self.zone = FocusZone::ActionBar;
                    self.action_index = 0;
                }
            }
        }
    }

    /// Returns the focused index for a zone, or None if that zone is not focused.
    pub fn focused_in(&self, zone: FocusZone) -> Option<usize> {
        if self.zone == zone {
            Some(match zone {
                FocusZone::Content => self.content_index,
                FocusZone::ActionBar => self.action_index,
                FocusZone::NavBar => self.nav_index,
            })
        } else {
            None
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

// INLINE_TEST_REQUIRED: FocusManager tests need access to private fields and internal state
#[cfg(test)]
mod tests {
    use super::*;

    fn setup(content: usize, action: usize, nav: usize) -> FocusManager {
        let mut fm = FocusManager::new();
        fm.set_counts(content, action, nav);
        fm
    }

    #[test]
    fn test_initial_state() {
        let fm = setup(5, 3, 5);
        assert_eq!(fm.zone, FocusZone::Content);
        assert_eq!(fm.content_index, 0);
    }

    #[test]
    fn test_down_past_content_goes_to_action_bar() {
        let mut fm = setup(2, 3, 5);
        fm.move_down(); // content_index 0 → 1
        assert_eq!(fm.content_index, 1);
        fm.move_down(); // past content → action bar
        assert_eq!(fm.zone, FocusZone::ActionBar);
    }

    #[test]
    fn test_down_from_action_bar_goes_to_nav_bar() {
        let mut fm = setup(1, 3, 5);
        fm.move_down(); // content → action bar
        assert_eq!(fm.zone, FocusZone::ActionBar);
        fm.move_down(); // action → nav bar
        assert_eq!(fm.zone, FocusZone::NavBar);
    }

    #[test]
    fn test_up_from_bars_returns_to_content() {
        let mut fm = setup(3, 2, 5);
        fm.zone = FocusZone::ActionBar;
        fm.move_up();
        assert_eq!(fm.zone, FocusZone::Content);

        fm.zone = FocusZone::NavBar;
        fm.move_up();
        assert_eq!(fm.zone, FocusZone::Content);
    }

    #[test]
    fn test_right_wraps_action_to_nav() {
        let mut fm = setup(1, 2, 5);
        fm.zone = FocusZone::ActionBar;
        fm.action_index = 1; // last action
        fm.move_right();
        assert_eq!(fm.zone, FocusZone::NavBar);
        assert_eq!(fm.nav_index, 0);
    }

    #[test]
    fn test_left_wraps_nav_to_action() {
        let mut fm = setup(1, 3, 5);
        fm.zone = FocusZone::NavBar;
        fm.nav_index = 0; // first nav
        fm.move_left();
        assert_eq!(fm.zone, FocusZone::ActionBar);
        assert_eq!(fm.action_index, 2); // last action
    }

    #[test]
    fn test_right_wraps_nav_to_action() {
        let mut fm = setup(1, 3, 5);
        fm.zone = FocusZone::NavBar;
        fm.nav_index = 4; // last nav
        fm.move_right();
        assert_eq!(fm.zone, FocusZone::ActionBar);
        assert_eq!(fm.action_index, 0);
    }

    #[test]
    fn test_left_wraps_action_to_nav() {
        let mut fm = setup(1, 3, 5);
        fm.zone = FocusZone::ActionBar;
        fm.action_index = 0; // first action
        fm.move_left();
        assert_eq!(fm.zone, FocusZone::NavBar);
        assert_eq!(fm.nav_index, 4); // last nav
    }

    #[test]
    fn test_focused_in_returns_correct_index() {
        let mut fm = setup(5, 3, 5);
        assert_eq!(fm.focused_in(FocusZone::Content), Some(0));
        assert_eq!(fm.focused_in(FocusZone::ActionBar), None);

        fm.zone = FocusZone::ActionBar;
        fm.action_index = 2;
        assert_eq!(fm.focused_in(FocusZone::ActionBar), Some(2));
        assert_eq!(fm.focused_in(FocusZone::Content), None);
    }

    #[test]
    fn test_content_up_at_top_is_noop() {
        let mut fm = setup(3, 2, 5);
        fm.move_up();
        assert_eq!(fm.content_index, 0);
        assert_eq!(fm.zone, FocusZone::Content);
    }

    #[test]
    fn test_set_counts_clamps_indices() {
        let mut fm = FocusManager::new();
        fm.content_index = 10;
        fm.action_index = 10;
        fm.set_counts(3, 2, 5);
        assert_eq!(fm.content_index, 2);
        assert_eq!(fm.action_index, 1);
    }
}
