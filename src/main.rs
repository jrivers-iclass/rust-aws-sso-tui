mod errors;
mod tui;
mod sso;
mod aws;
mod utils;
mod widgets;
mod app;

use app::*;
use color_eyre::Result;

fn main() -> Result<()> {
    errors::install_hooks()?;  
    let mut terminal = tui::init()?;
    App::default().run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}