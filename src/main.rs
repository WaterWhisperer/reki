mod app;
mod event;
mod git;
mod ui;

use anyhow::Result;

use app::App;
use event::EventHandler;
use ui::Tui;

fn main() -> Result<()> {
    let mut app = App::new()?;
    let mut tui = Tui::new()?;
    tui.enter()?;

    while !app.should_quit {
        tui.draw(&mut app)?;

        if let Some(ev) = EventHandler::poll()? {
            app.handle_event(ev);
        }
    }

    tui.exit()?;
    Ok(())
}
