use std::rc::Rc;

use ratatui::{    
    crossterm::event::{KeyCode, KeyEvent}, layout::{Alignment, Constraint, Layout, Rect}, style::{Style, Stylize}, symbols::border, text::Line, widgets::{
        block::{Position, Title}, Block, Cell, Row, Table
    }, Frame
};

use crate::app::App;
use super::Page;

#[derive(Clone)]
pub struct CredentialsPage;
impl Page for CredentialsPage {
    fn active(&self, app: App) -> bool {
        return app.role_is_selected == true;
    }

    fn handle_key_events(&mut self, app: &mut App, key: KeyEvent) -> Result<(), ()> {
        match key.code {
            KeyCode::Char('q') => app.exit(),                
            KeyCode::Char('c') => {            
                app.credential_message = "Opening AWS Console...".to_string();
                app.open_console()
            }       
            KeyCode::Char('e') => {            
                let _ = app.export();
            },
            KeyCode::Left => {            
                app.role_is_selected = false;     
            }
            _ => {}
        }
    
        Ok(())
    }

    fn get_layout(&mut self, frame: &Frame) -> Rc<[Rect]> {
        Layout::horizontal([Constraint::Min(5)]).split(frame.size())
    }

    fn render(&mut self, frame: &mut Frame, app: &mut App) {
        let rect = self.get_layout(frame)[0];
        let instructions = Title::from(Line::from(vec![
            " Back ".into(),
            "<Left>".blue().bold(),        
            " Console ".into(),
            "<C>".blue().bold(),
            " Export ".into(),
            "<E>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]));
        let title = Title::from(format!("Credentials for {} - {}", app.selected_account.account_name, app.selected_role).bold());        
        let block = Block::bordered()
            .title(title.alignment(Alignment::Left))   
            .title(instructions
                .alignment(Alignment::Center)
                .position(Position::Bottom)
            )        
            .border_set(border::THICK);
    
        let widths = [
            Constraint::Max(20),
            Constraint::Min(10),
        ];
    
        let rows = vec![
            Row::new(vec![
                Cell::from("Access Key ID"),
                Cell::from(app.role_credentials.access_key_id.clone())
            ]),
            Row::new(vec![
                Cell::from("Secret Access Key"),
                Cell::from(app.role_credentials.secret_access_key.clone())
            ]),
            Row::new(vec![
                Cell::from("Session Token"),
                Cell::from(app.role_credentials.session_token.clone())
            ]),
            Row::new(vec![
                Cell::from("Expiration"),
                Cell::from(app.role_credentials.expiration.clone())
            ]),
        ];
    
        // let mut binding = app.selected_account.clone();
        let table = Table::new(rows, widths)
            .column_spacing(1)
            .style(Style::new().blue())                              
            .footer(Row::new(vec!["".to_string(), app.credential_message.clone()]).bold().yellow())
            .block(block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>");
    
        frame.render_widget(table, rect);
    }    
}