/// A node in the heading tree built from markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub level: u8,
    pub title: String,
    pub body: String,
    pub children: Vec<Section>,
}

impl Section {
    pub fn child(&self, title: &str) -> Option<&Section> {
        self.children
            .iter()
            .find(|c| c.title.eq_ignore_ascii_case(title))
    }

    pub fn children_named(&self, title: &str) -> Vec<&Section> {
        self.children
            .iter()
            .filter(|c| c.title.eq_ignore_ascii_case(title))
            .collect()
    }

    pub fn body_trimmed(&self) -> &str {
        self.body.trim()
    }
}
