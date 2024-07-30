use std::rc::Rc;

use ratatui::{    
    crossterm::event::{KeyCode, KeyEvent}, layout::{Alignment, Constraint, Layout, Rect}, style::{Style, Stylize}, symbols::border, text::Line, widgets::{
        block::{Position, Title}, Block, Cell, Row, Table
    }, Frame
};

use crate::app::App;
use super::Page;

#[derive(Clone)]
pub struct RolesPage;
impl Page for RolesPage {
    fn handle_key_events(&mut self, app: &mut App, key: KeyEvent) -> Result<(), ()> {
        match key.code {
            KeyCode::Down => {
                app.next_role();
            }
            KeyCode::Up => {
                app.previous_role();
            }
            KeyCode::Left => {
                app.is_selected = false;
                app.current_page = crate::app::CurrentPage::AccountList;
            }
            KeyCode::Right => {
                app.select_role();
                app.current_page = crate::app::CurrentPage::Credentials;
            }        
            KeyCode::Char('q') => {
                app.exit = true;
            }
            _ => {}
        }
    
        Ok(())
    }

    fn get_layout(&mut self, frame: &Frame) -> Rc<[Rect]> {
        Layout::horizontal([Constraint::Min(5), Constraint::Min(5)]).split(frame.size())
    }

    fn render(&mut self, frame: &mut Frame, app: &mut App, rect: Rect) {
        let instructions = Title::from(Line::from(vec![
            " Scroll Up ".into(),
            "<Up>".blue().bold(),
            " Scroll Down ".into(),
            "<Down>".blue().bold(),
            " Select Role ".into(),        
            "<Enter>".blue().bold(),
            " Back ".into(),
            "<Left>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]));
        let role_list_title = Title::from(format!(" {} - Roles ", app.selected_account.account_name).bold());        
        let role_list_block = Block::bordered()
            .title(role_list_title.alignment(Alignment::Left))   
            .title(instructions
                .alignment(Alignment::Center)
                .position(Position::Bottom)
            )        
            .border_set(border::THICK);
    
        let widths = [
            Constraint::Min(10)
        ];
    
        let rows = app.selected_account.roles.iter().map(|row| {
            Row::new(vec![
                Cell::from(row.clone())
            ])
        });    
    
        // let mut binding = app.selected_account.clone();
        let table = Table::new(rows, widths)
            .column_spacing(1)
            .style(Style::new().blue())
            .header(
                Row::new(vec!["Role"])
                    .style(Style::new().bold())                            
            )                                
            //.footer(Row::new(vec!["Selected Account", &binding.account_id]).bold().yellow())
            .block(role_list_block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>");
    
        frame.render_stateful_widget(table, rect, &mut app.role_table_state);
    }    
}