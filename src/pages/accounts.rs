use std::rc::Rc;

use ratatui::{    
    crossterm::event::{KeyCode, KeyEvent}, layout::{Alignment, Constraint, Layout, Rect}, style::{Style, Stylize}, symbols::border, text::Line, widgets::{
        block::{Position, Title}, Block, Cell, Row, Table
    }, Frame
};

use crate::app::App;
use super::Page;

#[derive(Clone)]
pub struct AccountsPage;
impl Page for AccountsPage {
    fn active(&self, _app: App) -> bool {
        return true;
    }

    fn handle_key_events(&mut self, app: &mut App, key: KeyEvent) -> Result<(), ()> {
        match key.code {
            KeyCode::Down => {
                app.next();
            }
            KeyCode::Up => {
                app.previous();
            }
            KeyCode::Right => {
                app.select_account();
                app.current_page = crate::app::CurrentPage::Roles;
            }
            KeyCode::Char('c') => {
                app.currently_editing = true;
                app.current_page = crate::app::CurrentPage::Config;
            }
            KeyCode::Char('q') => {
                app.exit = true;
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
        let style = {
            if app.is_selected {
                Style::new().white()
            } else {
                Style::new().blue()
            }
        };
        let instructions = Title::from(Line::from(vec![
            " Scroll Up ".into(),
            "<Up>".blue().bold(),
            " Scroll Down ".into(),
            "<Down>".blue().bold(),
            " Select Account ".into(),
            "<Right>".blue().bold(),
            " Config ".into(),
            "<C>".yellow().bold(),
            " Quit ".into(),
            "<Q> ".red().bold(),
        ]));
        
        let start_url = app.config_options.options.iter().find(|option| option.name == "start_url").unwrap().value.clone();
        let url_title = Title::from(format!(" Start URL: {} ", start_url).bold());
    
        let account_list_title = Title::from(format!(" Accounts ({}) ", app.rows.len()).bold());        
        let account_list_block = Block::bordered()
            .title(account_list_title.alignment(Alignment::Left))   
            .title(instructions
                .alignment(Alignment::Center)
                .position(Position::Bottom)
            )   
            .title (url_title.alignment(Alignment::Right))     
            .border_set(border::THICK);
    
        let widths = [
            Constraint::Min(10),
            Constraint::Min(20)
        ];
    
        let rows = app.rows.iter().map(|row| {
            Row::new(vec![
                Cell::from(row.account_name.clone()),
                Cell::from(row.account_id.clone())
            ])
        });    
    
        let footer_row = Row::new(vec![
            Cell::from("Selected Account:").style(Style::new().bold()),
            Cell::from(app.selected_account.account_id.clone()).style(Style::new().bold().yellow())
        ]);    
    
        let table = Table::new(rows, widths)
            .column_spacing(1)
            .style(style)
            .header(
                Row::new(vec!["Account Name", "Account ID"])
                    .style(Style::new().bold())                            
            )                                
            .footer(footer_row)
            .block(account_list_block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>");
    
        frame.render_stateful_widget(table, rect, &mut app.table_state);
    }    
}