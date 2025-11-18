/// VIM editing modes for the input field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Insert mode - typing inserts characters
    Insert,
    /// Normal mode - VIM navigation and commands
    Normal,
}

impl Default for EditorMode {
    fn default() -> Self {
        EditorMode::Insert
    }
}

impl EditorMode {
    /// Get the display string for the mode indicator
    pub fn display(&self) -> &str {
        match self {
            EditorMode::Insert => "INSERT",
            EditorMode::Normal => "NORMAL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        assert_eq!(EditorMode::default(), EditorMode::Insert);
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(EditorMode::Insert.display(), "INSERT");
        assert_eq!(EditorMode::Normal.display(), "NORMAL");
    }
}
