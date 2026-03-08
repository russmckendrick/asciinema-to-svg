include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// Look up an icon by name, returning its SVG path data if found.
pub fn lookup(name: &str) -> Option<&'static str> {
    ICONS.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_icon_returns_some() {
        assert!(lookup("apple-fill").is_some());
        let data = lookup("apple-fill").unwrap();
        assert!(data.contains('M'), "path data should contain move commands");
    }

    #[test]
    fn unknown_icon_returns_none() {
        assert!(lookup("nonexistent-icon-xyz").is_none());
    }
}
