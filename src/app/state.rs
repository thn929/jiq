use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders},
};
use tui_textarea::TextArea;

use crate::editor::EditorMode;
use crate::query::executor::JqExecutor;

/// Which pane has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    InputField,
    ResultsPane,
}

/// What to output when exiting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Results, // Output filtered JSON results (Enter)
    Query,   // Output query string only (Shift+Enter)
}

/// Application state
pub struct App {
    pub textarea: TextArea<'static>,
    pub executor: JqExecutor,
    pub query_result: Result<String, String>,
    pub focus: Focus,
    pub editor_mode: EditorMode,
    pub results_scroll: u16,
    pub results_viewport_height: u16,
    pub output_mode: Option<OutputMode>,
    pub should_quit: bool,
}

impl App {
    /// Create a new App instance with JSON input
    pub fn new(json_input: String) -> Self {
        // Create textarea for query input
        let mut textarea = TextArea::default();

        // Configure for single-line input
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Query ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        // Remove default underline from cursor line
        textarea.set_cursor_line_style(Style::default());

        // Create JQ executor
        let executor = JqExecutor::new(json_input.clone());

        // Initial result text on startup
        let query_result = executor.execute(".");

        Self {
            textarea,
            executor,
            query_result,
            focus: Focus::InputField, // Start with input field focused
            editor_mode: EditorMode::default(), // Start in Insert mode
            results_scroll: 0,
            results_viewport_height: 0, // Will be set during first render
            output_mode: None, // No output mode set until Enter/Shift+Enter
            should_quit: false,
        }
    }

    /// Check if the application should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Get the output mode (if set)
    pub fn output_mode(&self) -> Option<OutputMode> {
        self.output_mode
    }

    /// Get the current query text
    pub fn query(&self) -> &str {
        self.textarea.lines()[0].as_ref()
    }

    /// Get the total number of lines in the current results
    pub fn results_line_count(&self) -> u16 {
        match &self.query_result {
            Ok(result) => result.lines().count() as u16,
            Err(error) => error.lines().count() as u16,
        }
    }

    /// Get the maximum scroll position based on content and viewport
    pub fn max_scroll(&self) -> u16 {
        let total_lines = self.results_line_count();
        total_lines.saturating_sub(self.results_viewport_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        let json = r#"{"name": "Alice", "age": 30}"#;
        let app = App::new(json.to_string());

        // Check default state
        assert_eq!(app.focus, Focus::InputField);
        assert_eq!(app.results_scroll, 0);
        assert_eq!(app.output_mode, None);
        assert!(!app.should_quit);
        assert_eq!(app.query(), "");
    }

    #[test]
    fn test_initial_query_result() {
        let json = r#"{"name": "Bob"}"#;
        let app = App::new(json.to_string());

        // Initial query should execute identity filter "."
        assert!(app.query_result.is_ok());
        let result = app.query_result.as_ref().unwrap();
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_focus_enum() {
        assert_eq!(Focus::InputField, Focus::InputField);
        assert_eq!(Focus::ResultsPane, Focus::ResultsPane);
        assert_ne!(Focus::InputField, Focus::ResultsPane);
    }

    #[test]
    fn test_output_mode_enum() {
        assert_eq!(OutputMode::Results, OutputMode::Results);
        assert_eq!(OutputMode::Query, OutputMode::Query);
        assert_ne!(OutputMode::Results, OutputMode::Query);
    }

    #[test]
    fn test_should_quit_getter() {
        let json = r#"{}"#;
        let mut app = App::new(json.to_string());

        assert!(!app.should_quit());

        app.should_quit = true;
        assert!(app.should_quit());
    }

    #[test]
    fn test_output_mode_getter() {
        let json = r#"{}"#;
        let mut app = App::new(json.to_string());

        assert_eq!(app.output_mode(), None);

        app.output_mode = Some(OutputMode::Results);
        assert_eq!(app.output_mode(), Some(OutputMode::Results));

        app.output_mode = Some(OutputMode::Query);
        assert_eq!(app.output_mode(), Some(OutputMode::Query));
    }

    #[test]
    fn test_query_getter_empty() {
        let json = r#"{"test": true}"#;
        let app = App::new(json.to_string());

        assert_eq!(app.query(), "");
    }

    #[test]
    fn test_app_with_empty_json_object() {
        let json = "{}";
        let app = App::new(json.to_string());

        assert!(app.query_result.is_ok());
    }

    #[test]
    fn test_app_with_json_array() {
        let json = r#"[1, 2, 3]"#;
        let app = App::new(json.to_string());

        assert!(app.query_result.is_ok());
        let result = app.query_result.as_ref().unwrap();
        assert!(result.contains("1"));
        assert!(result.contains("2"));
        assert!(result.contains("3"));
    }
}
