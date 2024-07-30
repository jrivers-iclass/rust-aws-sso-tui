use std::rc::Rc;

use ratatui::{    
    crossterm::event::{KeyCode, KeyEvent}, layout::{Alignment, Constraint, Layout, Rect}, style::{Style, Stylize}, symbols::border, text::Line, widgets::{
        block::{Position, Title}, Block, Row, Table
    }, Frame
};

use crate::{app::App, ConfigOption};
use super::Page;

#[derive(Clone)]
pub struct ConfigPage;
impl Page for ConfigPage {
    fn handle_key_events(&mut self, app: &mut App, key: KeyEvent) -> Result<(), ()> {
        match key.code {
            KeyCode::Enter => {
                app.currently_editing = false;
                let mut config = app.load_config().unwrap();
                app.config_options.options.iter().for_each(|option| {
                    config.with_section(Some("Main".to_string()))
                        .set(option.name.clone(), option.value.clone());                
                });
                app.update_config(&mut config).map_err(|err| {
                    anyhow::anyhow!("Failed to update config: {}", err)
                }).unwrap();            
    
                app.load_aws_config(Some(true));
                app.get_account_list();
                app.current_page = crate::app::CurrentPage::AccountList;
            },
            KeyCode::Down => {
                let i = match app.config_table_state.selected() {
                    Some(i) => {
                        if i >= app.config_options.options.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                app.config_table_state.select(Some(i));
                app.value_input = app.config_options.options[i].value.clone();
            },
            KeyCode::Up => {
                let i = match app.config_table_state.selected() {
                    Some(i) => {
                        if i <= 0 {
                            1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                app.config_table_state.select(Some(i));
                app.value_input = app.config_options.options[i].value.clone();
            },
            KeyCode::Char(value) => {                      
                if app.config_table_state.selected() != None {
                    let i = app.config_table_state.selected().unwrap();
                    app.config_options.options[i].value.push(value);
                }
            },
            KeyCode::Backspace => {
                if app.config_table_state.selected() != None {
                    let i = app.config_table_state.selected().unwrap();
                    app.config_options.options[i].value.pop();
                }
            },      
            KeyCode::Esc => {
                app.currently_editing = false;
                app.exit();
            },          
            _ => {}
        }
    
        Ok(())
    }

    fn get_layout(&mut self, frame: &Frame) -> Rc<[Rect]> {
        Layout::horizontal([Constraint::Min(5)]).split(frame.size())
    }

    fn render(&mut self, frame: &mut Frame, app: &mut App, rect: Rect) {
        let instructions = Title::from(Line::from(vec![
            " Next ".into(),
            "<Down>".blue().bold(),
            " Previous ".into(),
            "<Up>".blue().bold(),
            " Save ".into(),        
            "<Enter>".blue().bold(),       
            " Quit ".into(),
            "<Esc> ".blue().bold(),
        ]));
        let title = Title::from(" Config ".bold());        
        let block = Block::bordered()
            .title(title.alignment(Alignment::Center))   
            .title(instructions
                .alignment(Alignment::Center)
                .position(Position::Bottom)
            )        
            .border_set(border::THICK);
    
        let widths = [
            Constraint::Min(10),
            Constraint::Min(10)
        ];
    
        let rows = app.config_options.options.iter().map(|option: &ConfigOption| {
            Row::new(vec![option.name.clone(), option.value.clone()])
        });
    
        let mut footer_row = Row::new(vec!["", ""]);
        if !app.token_prompt.is_empty() {
            // TODO: Figure out why this isn't working
            footer_row = Row::new(vec!["AWS", &app.token_prompt]);
        }
    
        let table = Table::new(rows, widths)
            .column_spacing(1)
            .style(Style::new().blue())
            .header(
                Row::new(vec!["Key", "Value"])
                    .style(Style::new().bold())                            
            )       
            .footer(footer_row)                         
            .block(block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>");
    
        frame.render_stateful_widget(table, rect, &mut app.config_table_state);
    }    
}