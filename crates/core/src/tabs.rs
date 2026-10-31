//! Ordered tab list with one active tab. The shell maps each [`TabId`] to a browser view.

/// Stable identifier of a tab for the lifetime of the window.
pub type TabId = u64;

/// Title shown for a tab that has neither a title nor a URL yet.
pub const NEW_TAB_TITLE: &str = "New Tab";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub url: String,
}

impl Tab {
    /// The page title, else the URL, else [`NEW_TAB_TITLE`].
    pub fn display_title(&self) -> &str {
        [self.title.as_str(), self.url.as_str()]
            .into_iter()
            .find(|text| !text.is_empty())
            .unwrap_or(NEW_TAB_TITLE)
    }
}

#[derive(Debug, Default)]
pub struct Tabs {
    tabs: Vec<Tab>,
    active: Option<usize>,
    next_id: TabId,
}

impl Tabs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens an empty tab right after the active one (or at the end) and activates it.
    pub fn open(&mut self) -> TabId {
        self.next_id += 1;
        let id = self.next_id;
        let index = self.active.map_or(self.tabs.len(), |active| active + 1);
        self.tabs.insert(
            index,
            Tab {
                id,
                title: String::new(),
                url: String::new(),
            },
        );
        self.active = Some(index);
        id
    }

    /// Closes `id`. Closing the active tab activates its right neighbor, else its left one.
    /// Returns `false` (and changes nothing) when `id` is unknown.
    pub fn close(&mut self, id: TabId) -> bool {
        let Some(index) = self.index_of(id) else {
            return false;
        };
        self.tabs.remove(index);
        self.active = match self.active {
            _ if self.tabs.is_empty() => None,
            Some(active) if active > index => Some(active - 1),
            Some(active) if active == index => Some(index.min(self.tabs.len() - 1)),
            other => other,
        };
        true
    }

    /// Activates `id`. Returns `false` (and changes nothing) when `id` is unknown.
    pub fn activate(&mut self, id: TabId) -> bool {
        let index = self.index_of(id);
        if index.is_some() {
            self.active = index;
        }
        index.is_some()
    }

    /// Activates the next tab, wrapping around; returns the newly active tab.
    pub fn activate_next(&mut self) -> Option<TabId> {
        self.step(1)
    }

    /// Activates the previous tab, wrapping around; returns the newly active tab.
    pub fn activate_previous(&mut self) -> Option<TabId> {
        self.step(self.tabs.len().saturating_sub(1))
    }

    /// Updates a tab's title. Unknown ids are ignored.
    pub fn set_title(&mut self, id: TabId, title: &str) {
        if let Some(tab) = self.get_mut(id) {
            tab.title = title.to_owned();
        }
    }

    /// Updates a tab's URL. Unknown ids are ignored.
    pub fn set_url(&mut self, id: TabId, url: &str) {
        if let Some(tab) = self.get_mut(id) {
            tab.url = url.to_owned();
        }
    }

    pub fn active(&self) -> Option<&Tab> {
        self.active.and_then(|index| self.tabs.get(index))
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.iter()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    fn get_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    /// Moves the active index forward by `offset`, modulo the tab count.
    fn step(&mut self, offset: usize) -> Option<TabId> {
        let active = self.active?;
        let index = (active + offset) % self.tabs.len();
        self.active = Some(index);
        Some(self.tabs[index].id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(tabs: &Tabs) -> Vec<TabId> {
        tabs.iter().map(|tab| tab.id).collect()
    }

    fn active_id(tabs: &Tabs) -> Option<TabId> {
        tabs.active().map(|tab| tab.id)
    }

    /// Three tabs `[a, b, c]` with `b` active.
    fn three_with_middle_active() -> (Tabs, [TabId; 3]) {
        let mut tabs = Tabs::new();
        let a = tabs.open();
        let b = tabs.open();
        let c = tabs.open();
        assert!(tabs.activate(b));
        (tabs, [a, b, c])
    }

    #[test]
    fn a_new_list_is_empty() {
        let tabs = Tabs::new();
        assert!(tabs.is_empty());
        assert_eq!(tabs.len(), 0);
        assert_eq!(tabs.active(), None);
    }

    #[test]
    fn open_inserts_after_the_active_tab_and_activates_it() {
        let (mut tabs, [a, b, c]) = three_with_middle_active();
        let d = tabs.open();
        assert_eq!(ids(&tabs), vec![a, b, d, c]);
        assert_eq!(active_id(&tabs), Some(d));
        assert_eq!(tabs.len(), 4);
    }

    #[test]
    fn open_gives_unique_ids_and_empty_tabs() {
        let mut tabs = Tabs::new();
        let a = tabs.open();
        let b = tabs.open();
        assert_ne!(a, b);
        assert_eq!(tabs.get(a).map(|tab| tab.url.as_str()), Some(""));
        assert_eq!(tabs.get(a).map(|tab| tab.title.as_str()), Some(""));
    }

    #[test]
    fn closing_the_active_tab_activates_its_right_neighbor() {
        let (mut tabs, [a, b, c]) = three_with_middle_active();
        assert!(tabs.close(b));
        assert_eq!(ids(&tabs), vec![a, c]);
        assert_eq!(active_id(&tabs), Some(c));
    }

    #[test]
    fn closing_the_last_positioned_active_tab_activates_its_left_neighbor() {
        let (mut tabs, [a, b, c]) = three_with_middle_active();
        assert!(tabs.activate(c));
        assert!(tabs.close(c));
        assert_eq!(ids(&tabs), vec![a, b]);
        assert_eq!(active_id(&tabs), Some(b));
    }

    #[test]
    fn closing_an_inactive_tab_keeps_the_active_one() {
        let (mut tabs, [a, b, c]) = three_with_middle_active();
        assert!(tabs.close(a));
        assert_eq!(ids(&tabs), vec![b, c]);
        assert_eq!(active_id(&tabs), Some(b));
        assert!(tabs.close(c));
        assert_eq!(active_id(&tabs), Some(b));
    }

    #[test]
    fn closing_the_only_tab_leaves_the_list_empty() {
        let mut tabs = Tabs::new();
        let a = tabs.open();
        assert!(tabs.close(a));
        assert!(tabs.is_empty());
        assert_eq!(tabs.active(), None);
    }

    #[test]
    fn unknown_ids_change_nothing() {
        let (mut tabs, [a, b, c]) = three_with_middle_active();
        assert!(!tabs.close(99));
        assert!(!tabs.activate(99));
        tabs.set_title(99, "x");
        tabs.set_url(99, "https://x.dev");
        assert_eq!(ids(&tabs), vec![a, b, c]);
        assert_eq!(active_id(&tabs), Some(b));
        assert!(
            tabs.iter()
                .all(|tab| tab.title.is_empty() && tab.url.is_empty())
        );
    }

    #[test]
    fn next_and_previous_wrap_around() {
        let (mut tabs, [a, b, c]) = three_with_middle_active();
        assert_eq!(tabs.activate_next(), Some(c));
        assert_eq!(tabs.activate_next(), Some(a));
        assert_eq!(tabs.activate_previous(), Some(c));
        assert_eq!(tabs.activate_previous(), Some(b));
        assert_eq!(active_id(&tabs), Some(b));
    }

    #[test]
    fn next_and_previous_on_an_empty_list_return_none() {
        let mut tabs = Tabs::new();
        assert_eq!(tabs.activate_next(), None);
        assert_eq!(tabs.activate_previous(), None);
    }

    #[test]
    fn title_and_url_updates_apply_to_the_given_tab() {
        let (mut tabs, [a, b, _]) = three_with_middle_active();
        tabs.set_url(a, "https://rauchg.com/");
        tabs.set_title(a, "Guillermo Rauch");
        assert_eq!(
            tabs.get(a).map(|tab| tab.url.as_str()),
            Some("https://rauchg.com/")
        );
        assert_eq!(
            tabs.get(a).map(|tab| tab.title.as_str()),
            Some("Guillermo Rauch")
        );
        assert_eq!(tabs.get(b).map(|tab| tab.url.as_str()), Some(""));
    }

    #[test]
    fn display_title_falls_back_to_url_then_new_tab() {
        let mut tab = Tab {
            id: 1,
            title: String::new(),
            url: String::new(),
        };
        assert_eq!(tab.display_title(), NEW_TAB_TITLE);
        tab.url = "https://vercel.com/".into();
        assert_eq!(tab.display_title(), "https://vercel.com/");
        tab.title = "Vercel".into();
        assert_eq!(tab.display_title(), "Vercel");
    }
}
