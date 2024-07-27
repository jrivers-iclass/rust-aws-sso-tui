use ratatui::{    
    crossterm::event::{KeyCode, KeyEvent}, layout::{Alignment, Constraint, Rect}, style::{Style, Stylize}, symbols::border, text::Line, widgets::{
        block::{Position, Title}, Block, Row, Table
    }, Frame
};

use crate::app::App;

pub fn handle_key_events(app: &mut App, key: KeyEvent) -> Result<(), anyhow::Error>{
    match key.code {
        KeyCode::Enter => {
            app.start_url = app.value_input.clone();
            app.currently_editing = false;
            let mut config = app.load_config().unwrap();
            config.with_section(Some("Main".to_string()))
                .set("start_url", app.start_url.clone());
            app.update_config(&mut config).map_err(|err| {
                anyhow::anyhow!("Failed to update config: {}", err)
            })?;
            app.load_aws_config(Some(true));
            app.get_account_list();
            app.current_page = crate::app::CurrentPage::AccountList;
        },
        KeyCode::Char(value) => {
            app.value_input.push(value);
        },
        KeyCode::Backspace => {
            app.value_input.pop();
        },      
        KeyCode::Esc => {
            app.currently_editing = false;
            app.exit();
        },          
        _ => {}
    }

    Ok(())
}

pub fn render_config(f: &mut Frame, app: &mut App, area: Rect) {   
    let instructions = Title::from(Line::from(vec![
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
    
    if app.value_input.is_empty() {
        app.value_input = app.start_url.clone();
    }

    let rows = vec![
        Row::new(vec!["Start URL:", &app.value_input]),
    ];

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

    f.render_widget(table, area);
}