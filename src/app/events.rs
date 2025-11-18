use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io;
use tui_textarea::CursorMove;

use crate::editor::EditorMode;
use super::state::{App, Focus, OutputMode};

impl App {
    /// Handle events and update application state
    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // Check that it's a key press event to avoid duplicates
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle key press events
    fn handle_key_event(&mut self, key: KeyEvent) {
        // Try global keys first
        if self.handle_global_keys(key) {
            return; // Key was handled globally
        }

        // Not a global key, delegate to focused pane
        match self.focus {
            Focus::InputField => self.handle_input_field_key(key),
            Focus::ResultsPane => self.handle_results_pane_key(key),
        }
    }

    /// Handle global keys that work regardless of focus
    /// Returns true if key was handled, false otherwise
    fn handle_global_keys(&mut self, key: KeyEvent) -> bool {
        // Ctrl+C: Exit application
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return true;
        }

        // Tab: Switch focus between panes
        if key.code == KeyCode::Tab {
            self.focus = match self.focus {
                Focus::InputField => Focus::ResultsPane,
                Focus::ResultsPane => Focus::InputField,
            };
            return true;
        }

        // q: Exit application
        if key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return true;
        }

        // Shift+Enter (may be sent as Alt+Enter by some terminals): Exit and output query only
        if key.code == KeyCode::Enter
            && (key.modifiers.contains(KeyModifiers::SHIFT) || key.modifiers.contains(KeyModifiers::ALT))
        {
            self.output_mode = Some(OutputMode::Query);
            self.should_quit = true;
            return true;
        }

        // Enter: Exit and output filtered results
        if key.code == KeyCode::Enter {
            self.output_mode = Some(OutputMode::Results);
            self.should_quit = true;
            return true;
        }

        false // Key not handled
    }

    /// Handle keys when Input field is focused
    fn handle_input_field_key(&mut self, key: KeyEvent) {
        // Handle ESC - always switches to Normal mode
        if key.code == KeyCode::Esc {
            self.editor_mode = EditorMode::Normal;
            return;
        }

        // Handle input based on current mode
        match self.editor_mode {
            EditorMode::Insert => self.handle_insert_mode_key(key),
            EditorMode::Normal => self.handle_normal_mode_key(key),
            EditorMode::Operator(_) => self.handle_operator_mode_key(key),
        }
    }

    /// Handle keys in Insert mode
    fn handle_insert_mode_key(&mut self, key: KeyEvent) {
        // Use textarea's built-in input handling
        let content_changed = self.textarea.input(key);

        // Execute query on every keystroke that changes content
        if content_changed {
            let query = self.textarea.lines()[0].as_ref();
            self.query_result = self.executor.execute(query);
            // Reset scroll when query changes
            self.results_scroll = 0;
        }
    }

    /// Handle keys in Normal mode (VIM navigation and commands)
    fn handle_normal_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            // Basic cursor movement (h/l)
            KeyCode::Char('h') | KeyCode::Left => {
                self.textarea.move_cursor(CursorMove::Back);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.textarea.move_cursor(CursorMove::Forward);
            }

            // Line extent movement (0/$)
            KeyCode::Char('0') | KeyCode::Home => {
                self.textarea.move_cursor(CursorMove::Head);
            }
            KeyCode::Char('$') | KeyCode::End => {
                self.textarea.move_cursor(CursorMove::End);
            }

            // Word movement (w/b/e)
            KeyCode::Char('w') => {
                self.textarea.move_cursor(CursorMove::WordForward);
            }
            KeyCode::Char('b') => {
                self.textarea.move_cursor(CursorMove::WordBack);
            }
            KeyCode::Char('e') => {
                self.textarea.move_cursor(CursorMove::WordEnd);
            }

            // Enter Insert mode commands
            KeyCode::Char('i') => {
                // i - Insert at cursor
                self.editor_mode = EditorMode::Insert;
            }
            KeyCode::Char('a') => {
                // a - Append (insert after cursor)
                self.textarea.move_cursor(CursorMove::Forward);
                self.editor_mode = EditorMode::Insert;
            }
            KeyCode::Char('I') => {
                // I - Insert at line start
                self.textarea.move_cursor(CursorMove::Head);
                self.editor_mode = EditorMode::Insert;
            }
            KeyCode::Char('A') => {
                // A - Append at line end
                self.textarea.move_cursor(CursorMove::End);
                self.editor_mode = EditorMode::Insert;
            }

            _ => {
                // Other VIM commands not yet implemented
            }
        }
    }

    /// Handle keys in Operator mode (waiting for motion after d/c)
    fn handle_operator_mode_key(&mut self, _key: KeyEvent) {
        // TODO: Implement operator+motion system
        // For now, just cancel and return to Normal
        self.editor_mode = EditorMode::Normal;
    }

    /// Handle keys when Results pane is focused
    fn handle_results_pane_key(&mut self, key: KeyEvent) {
        match key.code {
            // Basic line scrolling (1 line)
            KeyCode::Up | KeyCode::Char('k') => {
                self.results_scroll = self.results_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.results_scroll = self.results_scroll.saturating_add(1);
            }

            // 10 line scrolling
            KeyCode::Char('K') => {
                self.results_scroll = self.results_scroll.saturating_sub(10);
            }
            KeyCode::Char('J') => {
                self.results_scroll = self.results_scroll.saturating_add(10);
            }

            // Jump to top
            KeyCode::Home | KeyCode::Char('g') => {
                self.results_scroll = 0;
            }

            // Jump to bottom
            KeyCode::Char('G') => {
                self.results_scroll = self.max_scroll();
            }

            // Half page scrolling up
            KeyCode::PageUp => {
                let half_page = self.results_viewport_height / 2;
                self.results_scroll = self.results_scroll.saturating_sub(half_page);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let half_page = self.results_viewport_height / 2;
                self.results_scroll = self.results_scroll.saturating_sub(half_page);
            }

            // Half page scrolling down
            KeyCode::PageDown => {
                let half_page = self.results_viewport_height / 2;
                self.results_scroll = self.results_scroll.saturating_add(half_page);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let half_page = self.results_viewport_height / 2;
                self.results_scroll = self.results_scroll.saturating_add(half_page);
            }

            _ => {
                // Ignore other keys in Results pane
            }
        }
    }
}
