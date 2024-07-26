use aws::AccountInfo;
use ratatui::{
    buffer::Buffer,
    crossterm::{
        cursor::position, event::{self, Event, KeyCode, KeyEvent, KeyEventKind}, style::Color // Add this line
    },
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{
        block::{Position, Title}, Block, Cell, Paragraph, Row, ScrollbarState, Table, TableState, Widget
    },
    Frame,
};

use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use sha1::digest::typenum::Length;

mod errors;
mod tui;
mod sso;
mod aws;
mod utils;

const ITEM_HEIGHT: usize = 4;

fn main() -> Result<()> {
    errors::install_hooks()?;
    let mut terminal = tui::init()?;
    App::default().run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Default, Clone)]
struct AccountRow {
    account_name: String,
    account_id: String,
    roles: Vec<String>,
}

#[derive(Default, Clone)]
pub struct App {
    table_state: TableState,
    rows: Vec<AccountRow>,
    exit: bool,    
    scroll_state: ScrollbarState,
    selected_account: AccountRow,
    role_table_state: TableState,
    is_selected: bool,
    role_is_selected: bool,
    selected_role: String,
    role_credentials: sso::RoleCredentials,
}

impl App {
    async fn new() -> Self {
        let mut rows = Vec::new();
        rows.push(AccountRow {
            account_name: "Loading...".to_string(),
            account_id: "".to_string(),
            roles: vec![],
        });        
        Self {
            table_state: TableState::default(),
            scroll_state: ScrollbarState::default(),
            exit: false,
            rows: rows,
            selected_account: AccountRow {
                account_name: "".to_string(),
                account_id: "".to_string(),
                roles: vec![],
            },
            is_selected: false,
            role_table_state: TableState::default(),
            selected_role: "".to_string(),
            role_credentials: sso::RoleCredentials {
                access_key_id: "".to_string(),
                secret_access_key: "".to_string(),
                session_token: "".to_string(),
                expiration: "".to_string(),
            },
            role_is_selected: false,
        }        
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {   
        let sso_accounts = sso::get_sso_accounts();
        match sso_accounts {
            Ok(sso_accounts) => {          
                for account in sso_accounts {
                    self.rows.push(AccountRow {
                        account_name: account.account_name,
                        account_id: account.account_id,
                        roles: account.roles,
                    });
                } 
            }
            Err(err) => {
                self.rows.push(AccountRow {
                    account_name: "Error".to_string(),
                    account_id: err.to_string(),
                    roles: vec![],
                });
            }
        }                
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events().wrap_err("handle events failed")?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {       
        let mut rects = Layout::vertical([
            Constraint::Min(5), 
            Constraint::Min(5)
            ]
        ).split(frame.size()); 
        if self.role_is_selected {
            render_credentials(frame, self, rects[0]);
        } else {
            if self.is_selected {
                rects = Layout::horizontal([
                    Constraint::Min(5), 
                    Constraint::Min(5)
                    ]
                ).split(frame.size());
            }
            render_accounts(frame,  self, rects[0]);
            if self.is_selected {
                render_roles(frame, self, rects[1]);
            }
        }

    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                .handle_key_event(key_event)
                .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}")),
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up => {
                if self.is_selected {
                    self.previous_role()
                } else {
                    self.previous()
                }
            },
            KeyCode::Down => {
                if self.is_selected {
                    self.next_role()
                } else {
                    self.next()
                }
            },
            KeyCode::Enter => self.open_console(),
            KeyCode::Right => {
                if self.is_selected {
                    self.select_role();
                } else {
                    self.select_account();
                }
            },           
            KeyCode::Left => {
                if self.role_is_selected {
                    self.role_is_selected = false;
                } else if self.is_selected {
                    self.is_selected = false;
                    self.role_table_state.select(None);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn open_console(&mut self) {
        if self.role_is_selected {
            let account_info = AccountInfo {
                account_name: self.selected_account.account_name.clone(),
                account_id: self.selected_account.account_id.clone(),
                roles: vec![],
            };
            let _ = sso::open_console(account_info, &self.selected_role);
        }
        ()
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    pub fn select_account(&mut self) {
        self.is_selected = true;
        let account_info = AccountInfo {
            account_name: self.selected_account.account_name.clone(),
            account_id: self.selected_account.account_id.clone(),
            roles: vec![],
        };
        let roles = match sso::get_account_roles(account_info) {
            Ok(roles) => roles,
            Err(err) => vec![err.to_string()],
        };
        self.selected_account.roles = roles;        
        self.selected_role = self.selected_account.roles[0].clone();
        self.role_table_state.select(Some(0));
    }

    pub fn select_role(&mut self) {
        let account_info = AccountInfo {
            account_name: self.selected_account.account_name.clone(),
            account_id: self.selected_account.account_id.clone(),
            roles: vec![],
        };
        let role = self.selected_role.clone();
        let role_credentials = match sso::get_account_role_credentials(account_info, &role) {
            Ok(role_credentials) => role_credentials,
            Err(err) => sso::RoleCredentials {
                access_key_id: "".to_string(),
                secret_access_key: "".to_string(),
                session_token: "".to_string(),
                expiration: err.to_string(),
            },
        };
        self.role_credentials = role_credentials;
        self.role_is_selected = true;
    }

    pub fn next_role(&mut self) {
        let i = match self.role_table_state.selected() {
            Some(i) => {
                if i >= self.selected_account.roles.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected_role = self.selected_account.roles[i].clone();
        self.role_table_state.select(Some(i));
        //self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_role(&mut self) {
        let i = match self.role_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.selected_account.roles.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_role = self.selected_account.roles[i].clone();
        self.role_table_state.select(Some(i));
        //self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.rows.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected_account = self.rows[i].clone();
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.rows.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_account = self.rows[i].clone();
        self.table_state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }
}

fn render_accounts(f: &mut Frame, app: &mut App, area: Rect) {
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

fn render_roles(f: &mut Frame, app: &mut App, area: Rect) {
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

fn render_credentials(f: &mut Frame, app: &mut App, area: Rect) {
    let instructions = Title::from(Line::from(vec![
        " Back ".into(),
        "<Left>".blue().bold(),        
        " Console ".into(),
        "<Enter>".blue().bold(),
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
        //.footer(Row::new(vec!["Selected Account", &binding.account_id]).bold().yellow())
        .block(block)
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>");

    f.render_widget(table, area);
}