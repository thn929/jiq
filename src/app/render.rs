use ansi_to_tui::IntoText;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::state::{App, Focus};

impl App {
    /// Render the UI
    pub fn render(&mut self, frame: &mut Frame) {
        // Split the terminal into three areas: results, input, and help
        let layout = Layout::vertical([
            Constraint::Min(3),      // Results pane takes most of the space
            Constraint::Length(3),   // Input field is fixed 3 lines
            Constraint::Length(1),   // Help line at bottom
        ])
        .split(frame.area());

        let results_area = layout[0];
        let input_area = layout[1];
        let help_area = layout[2];

        // Render results pane
        self.render_results_pane(frame, results_area);

        // Render input field
        self.render_input_field(frame, input_area);

        // Render help line
        self.render_help_line(frame, help_area);
    }

    /// Render the input field (bottom)
    fn render_input_field(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        // Set border color based on focus
        let border_color = if self.focus == Focus::InputField {
            Color::Cyan // Focused
        } else {
            Color::DarkGray // Unfocused
        };

        // Build title with mode indicator
        let title = format!(" Query [{}] ", self.editor_mode.display());

        // Update textarea block with focus-aware styling and mode indicator
        self.textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        );

        // Render the textarea widget
        frame.render_widget(&self.textarea, area);
    }

    /// Render the results pane (top)
    fn render_results_pane(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        // Store viewport height for page scrolling calculations (subtract borders)
        self.results_viewport_height = area.height.saturating_sub(2);

        // Set border color based on focus
        let border_color = if self.focus == Focus::ResultsPane {
            Color::Cyan // Focused
        } else {
            Color::DarkGray // Unfocused
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(Style::default().fg(border_color));

        // Display query results or error message
        let content = match &self.query_result {
            Ok(result) => {
                // Parse jq's ANSI color codes into Ratatui Text
                let colored_text = result
                    .as_bytes()
                    .to_vec()
                    .into_text()
                    .unwrap_or_else(|_| Text::raw(result)); // Fallback to plain text on parse error

                Paragraph::new(colored_text)
                    .block(block)
                    .scroll((self.results_scroll, 0))
            }
            Err(error) => {
                // Use red color for error messages
                Paragraph::new(error.as_str())
                    .block(block)
                    .style(Style::default().fg(Color::Red))
                    .scroll((self.results_scroll, 0))
            }
        };

        frame.render_widget(content, area);
    }

    /// Render the help line (bottom)
    fn render_help_line(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let help_text = " Tab: Switch Focus | Enter: Exit with Results | Shift+Enter: Exit with Query | q: Quit";

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(help, area);
    }
}
