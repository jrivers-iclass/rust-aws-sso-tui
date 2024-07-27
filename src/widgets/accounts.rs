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

pub fn render_accounts(f: &mut Frame, app: &mut App, area: Rect) {
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
        " Quit ".into(),
        "<Q> ".blue().bold(),
    ]));

    let account_list_title = Title::from(format!(" Accounts ({}) ", app.rows.len()).bold());        
    let account_list_block = Block::bordered()
        .title(account_list_title.alignment(Alignment::Left))   
        .title(instructions
            .alignment(Alignment::Center)
            .position(Position::Bottom)
        )        
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

    let binding = app.selected_account.clone();
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .style(style)
        .header(
            Row::new(vec!["Account Name", "Account ID"])
                .style(Style::new().bold())                            
        )                                
        .footer(Row::new(vec!["Selected Account", &binding.account_id]).bold().yellow())
        .block(account_list_block)
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>");

    f.render_stateful_widget(table, area, &mut app.table_state);
}