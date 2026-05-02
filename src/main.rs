use anyhow::Result;
use reki::{app::App, cli::Args, event::EventHandler, ui::Tui};

fn main() -> Result<()> {
    let args = Args::parse_from(std::env::args()).map_err(|err| anyhow::anyhow!("{err:?}"))?;
    let mut app = match args.repo_path {
        Some(path) => App::new_at(&path)?,
        None => App::new()?,
    };
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
