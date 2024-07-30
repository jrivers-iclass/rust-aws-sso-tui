use std::rc::Rc;

use ratatui::{crossterm::event::KeyEvent, layout::Rect, Frame};

use crate::App;
use crate::pages; // Add this line to import the 'pages' module

pub trait Page: Clone {
    fn handle_key_events(&mut self, app: &mut App, key: KeyEvent) -> Result<(), ()>;
    fn render(&mut self, frame: &mut Frame, app: &mut App, rect: Rect);
    fn get_layout(&mut self, frame: &Frame) -> Rc<[Rect]>;    
}

#[derive(Clone)]
pub enum PageEnum {
    ConfigPage(pages::ConfigPage),
    CredentialsPage(pages::CredentialsPage),
    RolesPage(pages::RolesPage),
    AccountsPage(pages::AccountsPage),
}

impl Page for PageEnum {
    fn handle_key_events(&mut self, app: &mut App, key: KeyEvent) -> Result<(), ()> {
        match self {
            PageEnum::ConfigPage(page) => page.handle_key_events(app, key),
            PageEnum::CredentialsPage(page) => page.handle_key_events(app, key),
            PageEnum::RolesPage(page) => page.handle_key_events(app, key),
            PageEnum::AccountsPage(page) => page.handle_key_events(app, key),
        }
    }

    fn render(&mut self, frame: &mut Frame, app: &mut App, rect: Rect) {
        match self {
            PageEnum::ConfigPage(page) => page.render(frame, app, rect),
            PageEnum::CredentialsPage(page) => page.render(frame, app, rect),
            PageEnum::RolesPage(page) => page.render(frame, app, rect),
            PageEnum::AccountsPage(page) => page.render(frame, app, rect),
        }
    }

    fn get_layout(&mut self, frame: &Frame) -> Rc<[Rect]> {
        match self {
            PageEnum::ConfigPage(page) => page.get_layout(frame),
            PageEnum::CredentialsPage(page) => page.get_layout(frame),
            PageEnum::RolesPage(page) => page.get_layout(frame),
            PageEnum::AccountsPage(page) => page.get_layout(frame),
        }
    }
}