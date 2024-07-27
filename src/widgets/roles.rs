use ratatui::{    
    layout::{Alignment, Constraint, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{
        block::{Position, Title}, Block, Cell, Row, Table
    },
    Frame,
};

use crate::app::App;

pub fn render_roles(f: &mut Frame, app: &mut App, area: Rect) {
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

    f.render_stateful_widget(table, area, &mut app.role_table_state);
}