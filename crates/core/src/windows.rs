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
}
