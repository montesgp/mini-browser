//! The app's top-level windows: each window's state by [`WindowId`], in the order the windows
//! opened, and which one the user activated last. The shell keeps one entry per window.

/// Stable identifier of a window for the lifetime of the app.
pub type WindowId = u64;

#[derive(Debug)]
pub struct Windows<S> {
    /// In the order the windows opened.
    entries: Vec<(WindowId, S)>,
    /// From the least to the most recently active window.
    recent: Vec<WindowId>,
    next_id: WindowId,
}

impl<S> Default for Windows<S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            recent: Vec::new(),
            next_id: 0,
        }
    }
}

impl<S> Windows<S> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a window with `state`; a new window is the most recently active one.
    pub fn add(&mut self, state: S) -> WindowId {
        self.next_id += 1;
        let id = self.next_id;
        self.entries.push((id, state));
        self.recent.push(id);
        id
    }

    /// Removes window `id` and returns its state; `None` when it is unknown.
    pub fn remove(&mut self, id: WindowId) -> Option<S> {
        let index = self.entries.iter().position(|(known, _)| *known == id)?;
        self.recent.retain(|known| *known != id);
        Some(self.entries.remove(index).1)
    }

    pub fn get(&self, id: WindowId) -> Option<&S> {
        self.entries
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, state)| state)
    }

    /// Makes `id` the most recently active window. Returns `false` when it is unknown.
    pub fn activate(&mut self, id: WindowId) -> bool {
        let Some(index) = self.recent.iter().position(|known| *known == id) else {
            return false;
        };
        self.recent.remove(index);
        self.recent.push(id);
        true
    }

    /// The window the user activated last, else the newest one; `None` without windows.
    pub fn last_active(&self) -> Option<WindowId> {
        self.recent.last().copied()
    }

    /// Every window's id, in the order the windows opened.
    pub fn ids(&self) -> Vec<WindowId> {
        self.entries.iter().map(|(id, _)| *id).collect()
    }

    /// The first window, in opening order, whose state matches `predicate`.
    pub fn find(&self, mut predicate: impl FnMut(&S) -> bool) -> Option<WindowId> {
        self.entries
            .iter()
            .find(|(_, state)| predicate(state))
            .map(|(id, _)| *id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The window a tab of window `from` returns to: `origin`, the window `from` was opened
    /// from by a move, while it is open; else the most recently active other window. `None`
    /// for a window not opened by a move (no `origin`) and when no other window is open.
    pub fn return_target(&self, from: WindowId, origin: Option<WindowId>) -> Option<WindowId> {
        self.return_target_skipping(from, origin, |_| false)
    }

    /// [`Windows::return_target`], never choosing a window for which `closing` is true: a tab
    /// joining a window that is closing would never be asked to close.
    pub fn return_target_skipping(
        &self,
        from: WindowId,
        origin: Option<WindowId>,
        closing: impl Fn(WindowId) -> bool,
    ) -> Option<WindowId> {
        origin?;
        origin
            .filter(|origin| *origin != from && self.get(*origin).is_some() && !closing(*origin))
            .or_else(|| {
                self.recent
                    .iter()
                    .rev()
                    .copied()
                    .find(|known| *known != from && !closing(*known))
            })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Whether the app quits after a browser closed: once no browser is left and no window keeps
/// running (open and not closing). A window whose closed last tab is being replaced by a home
/// tab has no browser for a moment but keeps running.
pub fn app_quits(browsers_left: usize, windows_running: usize) -> bool {
    browsers_left == 0 && windows_running == 0
}

/// Whether the active tab can move to a new window: not when it is its window's only tab, as
/// in Chrome, since the new window would only replace the old one.
pub fn can_move_to_new_window(tabs_in_window: usize) -> bool {
    tabs_in_window > 1
}

/// Whether dragging a tab out of its window's strip moves the whole window instead of tearing
/// the tab off: it does for the window's only tab, as in Chrome, since a new window would only
/// replace the old one.
pub fn dragging_moves_window(tabs_in_window: usize) -> bool {
    tabs_in_window == 1
}

/// Whether a window left without tabs (its last tab closed or moved to another window) closes:
/// it does while `other_windows` stay open; the app's only window gets a home tab instead.
pub fn closes_without_tabs(other_windows: usize) -> bool {
    other_windows > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activating_a_removed_window_changes_nothing() {
        let (mut windows, [a, b, _c]) = three();
        windows.remove(a);
        assert!(!windows.activate(a));
        windows.activate(b);
        assert_eq!(windows.last_active(), Some(b));
    }

    #[test]
    fn a_tab_never_returns_to_a_closing_window() {
        let (mut windows, [a, b, c]) = three();
        windows.activate(b);
        windows.activate(a);
        // The origin `a` is closing: the tab goes to the last active other window, `b`.
        assert_eq!(
            windows.return_target_skipping(c, Some(a), |w| w == a),
            Some(b)
        );
        // Every other window is closing: nowhere to go.
        assert_eq!(windows.return_target_skipping(c, Some(a), |w| w != c), None);
        // Nothing closing: the origin.
        assert_eq!(
            windows.return_target_skipping(c, Some(a), |_| false),
            Some(a)
        );
    }

    fn three() -> (Windows<&'static str>, [WindowId; 3]) {
        let mut windows = Windows::new();
        let a = windows.add("a");
        let b = windows.add("b");
        let c = windows.add("c");
        (windows, [a, b, c])
    }

    #[test]
    fn a_new_registry_is_empty() {
        let windows: Windows<()> = Windows::new();
        assert!(windows.is_empty());
        assert_eq!(windows.len(), 0);
        assert_eq!(windows.last_active(), None);
        assert!(windows.ids().is_empty());
    }

    #[test]
    fn added_windows_get_distinct_ids_and_keep_their_state() {
        let (windows, [a, b, c]) = three();
        assert_eq!(windows.len(), 3);
        assert!(a != b && b != c && a != c);
        assert_eq!(windows.get(a), Some(&"a"));
        assert_eq!(windows.get(c), Some(&"c"));
        assert_eq!(windows.ids(), vec![a, b, c]);
    }

    #[test]
    fn the_newest_window_is_the_last_active_until_another_is_activated() {
        let (mut windows, [a, b, c]) = three();
        assert_eq!(windows.last_active(), Some(c));
        assert!(windows.activate(a));
        assert_eq!(windows.last_active(), Some(a));
        assert!(windows.activate(b));
        assert_eq!(windows.last_active(), Some(b));
        // Activating changes which window is recent, not the opening order.
        assert_eq!(windows.ids(), vec![a, b, c]);
    }

    #[test]
    fn removing_the_last_active_window_falls_back_to_the_one_active_before() {
        let (mut windows, [a, b, c]) = three();
        windows.activate(a);
        windows.activate(b);
        assert_eq!(windows.remove(b), Some("b"));
        assert_eq!(windows.last_active(), Some(a));
        assert_eq!(windows.get(b), None);
        assert_eq!(windows.ids(), vec![a, c]);
        windows.remove(a);
        assert_eq!(windows.last_active(), Some(c));
        windows.remove(c);
        assert_eq!(windows.last_active(), None);
        assert!(windows.is_empty());
    }

    #[test]
    fn unknown_ids_change_nothing() {
        let (mut windows, [a, b, c]) = three();
        let unknown = a + b + c + 1;
        assert!(!windows.activate(unknown));
        assert_eq!(windows.last_active(), Some(c));
        assert_eq!(windows.remove(unknown), None);
        assert_eq!(windows.get(unknown), None);
        assert_eq!(windows.ids(), vec![a, b, c]);
    }

    #[test]
    fn ids_are_not_reused_after_a_window_closes() {
        let (mut windows, [a, b, c]) = three();
        windows.remove(c);
        let d = windows.add("d");
        assert!(![a, b, c].contains(&d));
        assert_eq!(windows.last_active(), Some(d));
    }

    #[test]
    fn find_returns_the_first_matching_window_in_opening_order() {
        let mut windows = Windows::new();
        let a = windows.add(1);
        let b = windows.add(2);
        let _c = windows.add(2);
        assert_eq!(windows.find(|state| *state == 1), Some(a));
        assert_eq!(windows.find(|state| *state == 2), Some(b));
        assert_eq!(windows.find(|state| *state == 3), None);
    }

    #[test]
    fn the_app_quits_only_without_browsers_and_running_windows() {
        assert!(app_quits(0, 0));
        // A window replacing its closed last tab with a home tab keeps the app running.
        assert!(!app_quits(0, 1));
        // Another window's browsers are still open.
        assert!(!app_quits(2, 0));
        assert!(!app_quits(1, 1));
    }

    #[test]
    fn a_moved_tab_returns_to_its_origin_while_it_is_open() {
        let (mut windows, [a, b, c]) = three();
        windows.activate(b);
        assert_eq!(windows.return_target(c, Some(a)), Some(a));
        assert_eq!(windows.return_target(c, Some(b)), Some(b));
    }

    #[test]
    fn without_its_origin_a_tab_returns_to_the_last_active_other_window() {
        let (mut windows, [a, b, c]) = three();
        windows.remove(a);
        windows.activate(b);
        windows.activate(c);
        // `c` itself is the most recent one; the tab goes to the window active before it.
        assert_eq!(windows.return_target(c, Some(a)), Some(b));
        // An origin that is the window itself is not a target either.
        assert_eq!(windows.return_target(c, Some(c)), Some(b));
        windows.activate(b);
        assert_eq!(windows.return_target(c, Some(a)), Some(b));
    }

    #[test]
    fn a_tab_has_nowhere_to_return_alone_or_outside_a_moved_window() {
        let mut windows = Windows::new();
        let a = windows.add(());
        assert_eq!(windows.return_target(a, Some(a + 1)), None);
        assert_eq!(windows.return_target(a, None), None);
        let b = windows.add(());
        // `a` was not opened by a move.
        assert_eq!(windows.return_target(a, None), None);
        assert_eq!(windows.return_target(b, Some(a)), Some(a));
    }

    #[test]
    fn only_a_window_with_other_tabs_can_move_its_tab_to_a_new_window() {
        assert!(!can_move_to_new_window(0));
        assert!(!can_move_to_new_window(1));
        assert!(can_move_to_new_window(2));
        assert!(can_move_to_new_window(9));
    }

    #[test]
    fn dragging_a_windows_only_tab_moves_the_window() {
        assert!(dragging_moves_window(1));
        assert!(!dragging_moves_window(2));
        assert!(!dragging_moves_window(9));
        // Exactly the tabs that cannot move to a new window, apart from an empty window.
        for tabs in 1..5 {
            assert_eq!(dragging_moves_window(tabs), !can_move_to_new_window(tabs));
        }
    }

    #[test]
    fn a_window_without_tabs_closes_only_while_other_windows_stay_open() {
        assert!(!closes_without_tabs(0));
        assert!(closes_without_tabs(1));
        assert!(closes_without_tabs(3));
    }
}
