//! Help Screen
//!
//! Displays keyboard shortcuts and FAQ from vauchi-core.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use vauchi_core::help::{get_faqs, HelpCategory};

use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, _app: &App) {
    // Build help text with keyboard shortcuts and FAQ summary
    let mut help_text = String::from(
        r#"Vauchi TUI Help
================

KEYBOARD SHORTCUTS
------------------
Navigation
  j/↓     Move down
  k/↑     Move up
  h/←     Move left (in dialogs)
  l/→     Move right (in dialogs)
  Enter   Select/confirm
  Esc     Go back/cancel
  Tab     Next field (in forms)

Home Screen
  e       Open exchange (QR code)
  c       View contacts
  s       Open settings
  a       Add new field
  d       Delete selected field

Contacts Screen
  Enter   View contact details

General
  ?       Show this help
  q       Quit


FAQ HIGHLIGHTS
--------------
"#,
    );

    // Add top FAQ items from each category
    let faqs = get_faqs();
    let categories = [
        HelpCategory::GettingStarted,
        HelpCategory::Privacy,
        HelpCategory::Recovery,
    ];

    for category in categories {
        if let Some(faq) = faqs.iter().find(|f| f.category == category) {
            help_text.push_str(&format!("\n• {}\n", faq.question));
            // Truncate answer for display
            let answer = if faq.answer.len() > 100 {
                format!("{}...", &faq.answer[..100])
            } else {
                faq.answer.clone()
            };
            help_text.push_str(&format!("  {}\n", answer.replace('\n', " ")));
        }
    }

    help_text.push_str("\nPress Esc or q to close this help screen.\n");

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true })
        .block(Block::default().title("Help").borders(Borders::ALL));

    f.render_widget(help, area);
}

/// Get all FAQ items for display.
#[allow(dead_code)]
pub fn get_all_faqs() -> Vec<(String, String, String)> {
    get_faqs()
        .into_iter()
        .map(|faq| (faq.id, faq.question, faq.answer))
        .collect()
}

/// Search FAQs by query.
#[allow(dead_code)]
pub fn search_help(query: &str) -> Vec<(String, String, String)> {
    vauchi_core::help::search_faqs(query)
        .into_iter()
        .map(|faq| (faq.id, faq.question, faq.answer))
        .collect()
}
