//! Interactive terminal UI for invy.
//!
//! Browse the place tree, and add, rename, describe, move or remove items
//! without leaving the app.

mod app;
mod input;
mod ui;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use std::path::Path;

use app::App;

/// Start the TUI against the given database.
pub fn run(db_path: Option<&Path>) -> Result<()> {
    let mut app = App::new(db_path)?;

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();

    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        // A resize also wakes this read, so the next draw picks up the new size.
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.on_key(key);
            }
        }
    }

    Ok(())
}
