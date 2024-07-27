use aws::AccountInfo;
use directories::UserDirs;
use ini::Ini;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout},
    widgets::{
       ScrollbarState, TableState
    },
    Frame,
};

use color_eyre::{
    eyre::{Error, WrapErr},
    Result,
};

mod errors;
mod tui;
mod sso;
mod aws;
mod utils;
mod widgets;
mod app;

use app::*;
use sso::ConfigProvider;

const ITEM_HEIGHT: usize = 4;

fn main() -> Result<()> {
    errors::install_hooks()?;  
    let mut terminal = tui::init()?;
    App::new().run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

fn load_config() -> Result<Ini, Error> {
    let file_path = UserDirs::new().unwrap().home_dir().join(".rust-tui").join("config.ini");

    let mut config = Ini::new();
    if !file_path.exists() {
        let _ = std::fs::create_dir_all(file_path.parent().unwrap());
        let _ = std::fs::write(file_path.clone(), "".as_bytes());

        config.with_section(Some("Main".to_string()))
            .set("start_url", "");

        update_config(&mut config)?;
    } else {
        config = Ini::load_from_file(file_path.clone())?;
    }

    Ok(config)
}

fn update_config(config: &mut Ini) -> Result<(), Error> {
    let file_path = UserDirs::new().unwrap().home_dir().join(".rust-tui").join("config.ini");
    config.write_to_file(file_path)?;
    Ok(())
}



impl App {
    fn new() -> Self {
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
            credential_message: "".to_string(),
            aws_config_provider: sso::ConfigProvider::default(),
            start_url: "".to_string(),
            value_input: "".to_string(),
            currently_editing: false,
            token_prompt: "".to_string(),
        }        
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {   
        let config = load_config()?;    

        self.start_url = config.get_from(Some("Main"), "start_url").unwrap().to_string();        

        if self.start_url.is_empty() {
            self.currently_editing = true;
        }  

        self.load_aws_config(Some(false));      

        self.get_account_list()        ;
                      
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events().wrap_err("handle events failed")?;
        }
        Ok(())
    }

    fn load_aws_config(&mut self, new_token: Option<bool>) {
        self.aws_config_provider = match sso::get_aws_config(self.start_url.clone().as_str(), self, Some(new_token.unwrap_or(false))) {
            Ok(access_token) => access_token,
            Err(_) => ConfigProvider::default(),
        };
    }

    fn get_account_list(&mut self) {
        if !self.aws_config_provider.account_info_provider.is_none() {
            let sso_accounts = sso::get_sso_accounts(self.aws_config_provider.clone());
            self.rows = vec![];
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
        } else {
            self.rows = vec![];
            self.rows.push(AccountRow {
                account_name: "Error".to_string(),
                account_id: "No AWS Config Provider".to_string(),
                roles: vec![],
            });
        }
    }

    fn render_frame(&mut self, frame: &mut Frame) {       
        let mut rects = Layout::vertical([
            Constraint::Min(5), 
            Constraint::Min(5)
            ]
        ).split(frame.size()); 

        if self.currently_editing {
            widgets::render_config(frame, self, rects[0]);
        } else {
            if self.role_is_selected {
                widgets::render_credentials(frame, self, rects[0]);
            } else {
                if self.is_selected {
                    rects = Layout::horizontal([
                        Constraint::Min(5), 
                        Constraint::Min(5)
                        ]
                    ).split(frame.size());
                }
                widgets::render_accounts(frame,  self, rects[0]);
                if self.is_selected {
                    widgets::render_roles(frame, self, rects[1]);
                }
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
        if self.currently_editing {
            match key_event.code {
                KeyCode::Enter => {
                    self.start_url = self.value_input.clone();
                    self.currently_editing = false;
                    let mut config = load_config()?;
                    config.with_section(Some("Main".to_string()))
                        .set("start_url", self.start_url.clone());
                    update_config(&mut config)?;
                    self.load_aws_config(Some(true));
                    self.get_account_list();
                },
                KeyCode::Char(value) => {
                    self.value_input.push(value);
                },
                KeyCode::Backspace => {
                    self.value_input.pop();
                },      
                KeyCode::Esc => {
                    self.currently_editing = false;
                    self.exit();
                },          
                _ => {}
            }
        } else {
            self.credential_message = "".to_string();
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
                KeyCode::Char('c') => {
                    if !self.role_is_selected && !self.is_selected {
                        self.currently_editing = true;
                    }
                    if self.role_is_selected {
                        self.credential_message = "Opening AWS Console...".to_string();
                        self.open_console()
                    }
                }
                KeyCode::Right => {
                    if self.is_selected {
                        self.select_role();
                    } else {
                        self.select_account();
                    }
                },         
                KeyCode::Char('e') => {
                    if !self.currently_editing && !self.role_is_selected && !self.is_selected {
                        self.start_url = "https://".to_string();
                        self.currently_editing = true;
                    } else if self.role_is_selected {
                        let _ = self.export();
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
            let _ = sso::open_console(self.role_credentials.clone(), account_info, &self.selected_role);
            self.credential_message += "Done!";
        }
        ()
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    pub fn export(&mut self) {
        #[cfg(target_os = "windows")]
        {
            self.credential_message = "Setting environment variables for AWS CLI - Windows...".to_string();
        }

        #[cfg(target_os = "macos")]
        {
            self.credential_message = "Copied environment variable exports for AWS CLI - MacOS...".to_string();
        }

        #[cfg(target_os = "linux")]
        {
            self.credential_message = "Copied environment variable exports for AWS CLI - Linux...".to_string();
        }

        let _ = sso::export_env_vars(&self.role_credentials);
        self.credential_message += "Done!";
    }

    pub fn select_account(&mut self) {
        self.is_selected = true;
        let account_info = AccountInfo {
            account_name: self.selected_account.account_name.clone(),
            account_id: self.selected_account.account_id.clone(),
            roles: vec![],
        };
        let roles = match sso::get_account_roles(self.aws_config_provider.clone(), account_info) {
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
        let role_credentials = match sso::get_account_role_credentials(self.aws_config_provider.clone(), account_info, &role) {
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