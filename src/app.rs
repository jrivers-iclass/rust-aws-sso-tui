use crate::{aws::AccountInfo, pages::{self, Page, PageEnum}, sso, tui};
use directories::UserDirs;
use ini::Ini;
use ratatui::{
    crossterm::event::{self, Event, KeyEvent, KeyEventKind}, widgets::{
        ScrollbarState, TableState,
    }, Frame
};
use color_eyre::{
    eyre::{Error, WrapErr},
    Result,
};
use crate::sso::{ConfigProvider, RoleCredentials};

const ITEM_HEIGHT: usize = 4;

#[derive(Default, Clone)]
pub struct AccountRow {
    pub account_name: String,
    pub account_id: String,
    pub roles: Vec<String>,
}

#[derive(Clone)]
pub struct ConfigOption {
    pub name: String,
    pub value: String,
}

#[derive(Clone)]
pub struct ConfigOptions {
    pub options: Vec<ConfigOption>,
}

#[derive(Clone)]
pub struct App {
    pub table_state: TableState,
    pub rows: Vec<AccountRow>,
    pub exit: bool,    
    pub scroll_state: ScrollbarState,
    pub selected_account: AccountRow,
    pub role_table_state: TableState,
    pub is_selected: bool,
    pub role_is_selected: bool,
    pub selected_role: String,
    pub role_credentials: RoleCredentials,
    pub credential_message: String,
    pub aws_config_provider: ConfigProvider,
    pub config_table_state: TableState,
    pub value_input: String,
    pub currently_editing: bool,
    pub token_prompt: String,
    pub routes: Vec<PageEnum>,
    pub config_options: ConfigOptions,
}

impl Default for App {
    fn default() -> Self {
        App {
            table_state: TableState::default(),
            rows: vec![],
            exit: false,
            scroll_state: ScrollbarState::default(),
            selected_account: AccountRow::default(),
            role_table_state: TableState::default(),
            is_selected: false,
            role_is_selected: false,
            selected_role: String::new(),
            role_credentials: RoleCredentials::default(),
            credential_message: String::new(),
            aws_config_provider: ConfigProvider::default(),
            config_table_state: TableState::default(),
            value_input: String::new(),
            currently_editing: false,
            token_prompt: String::new(),
            routes: vec![
                PageEnum::ConfigPage(pages::ConfigPage),                
                PageEnum::RolesPage(pages::RolesPage),
                PageEnum::CredentialsPage(pages::CredentialsPage),                
                PageEnum::AccountsPage(pages::AccountsPage),                
            ],
            config_options: ConfigOptions {
                options: vec![],
            },
        }
    }
}

impl App {    
    pub fn load_config(&mut self) -> Result<Ini, Error> {
        let file_path = UserDirs::new().unwrap().home_dir().join(".assumer").join("config.ini");
    
        let mut config = Ini::new();
        if !file_path.exists() {
            let _ = std::fs::create_dir_all(file_path.parent().unwrap());
            let _ = std::fs::write(file_path.clone(), "".as_bytes());
            
            self.config_options.options.iter().for_each(|option| {
                config.with_section(Some("Main".to_string()))
                    .set(option.name.clone(), option.value.clone());                
            });
    
            self.update_config(&mut config)?;
        } else {
            config = Ini::load_from_file(file_path.clone())?;
        }
    
        Ok(config)
    }
    
    pub fn update_config(&mut self, config: &mut Ini) -> Result<(), Error> {
        let file_path = UserDirs::new().unwrap().home_dir().join(".assumer").join("config.ini");
        config.write_to_file(file_path)?;
        Ok(())
    }     

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {   

        self.config_options = ConfigOptions {
            options: vec![
                ConfigOption {
                    name: "start_url".to_string(),
                    value: "".to_string(),
                },
                ConfigOption {
                    name: "aws_config_path".to_string(),
                    value: sso::get_default_aws_path().to_str().unwrap().to_string(),
                },
                ConfigOption {
                    name: "region".to_string(),
                    value: "us-east-1".to_string(),
                },
            ],
        };
        let config = self.load_config()?;
        

        // Map values from config to config_options
        for option in self.config_options.options.iter_mut() {
            let section = config.section(Some("Main".to_string())).unwrap();
            option.value = match section.get(&option.name) {
                Some(value) => value.to_string(),
                None => option.value.clone(),                
            }
        }      
        self.load_aws_config(Some(false));      

        self.get_account_list();
                      
        while !self.exit {          
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events().wrap_err("handle events failed")?;
        }
        Ok(())
    }

    pub fn load_aws_config(&mut self, new_token: Option<bool>) {
        let start_url = self.config_options.options.iter().find(|option| option.name == "start_url").unwrap().value.clone();
        let region = self.config_options.options.iter().find(|option| option.name == "region").unwrap().value.clone();

        self.aws_config_provider = match sso::get_aws_config(start_url.as_str(), region.as_str(), self, Some(new_token.unwrap_or(false))) {
            Ok(access_token) => access_token,
            Err(_) => ConfigProvider::default(),
        };
    }

    pub fn get_account_list(&mut self) {
        if !self.aws_config_provider.account_info_provider.is_none() {
            let sso_accounts = sso::get_sso_accounts(self);
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
        let state = self.clone();
        for route in &mut self.routes.iter_mut() {            
            if route.active(state.clone()) {
                // TODO: Fix borrow issue with self
                route.render(frame, self);
            }
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        let state = self.clone();
        for route in &mut self.routes.iter_mut() {            
            if route.active(state.clone()) {
                // TODO: Fix borrow issue with self
                route.handle_key_events(self, key_event);

                // We break here to prevent multiple routes from handling the same key event
                break;
            }
        }
        Ok(())
    }

    /// updates the application's self based on user input
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

    pub fn open_console(&mut self) {
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

    pub fn exit(&mut self) {
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

        let aws_config_path = self.config_options.options.iter().find(|option| option.name == "aws_config_path").unwrap().clone();
        let _ = sso::export_env_vars(&self.role_credentials, aws_config_path);
        self.credential_message += "Done!";
    }

    pub fn select_account(&mut self) {
        self.is_selected = true;
        let account_info = AccountInfo {
            account_name: self.selected_account.account_name.clone(),
            account_id: self.selected_account.account_id.clone(),
            roles: vec![],
        };
        let roles = match sso::get_account_roles(self, account_info) {
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
        let role_credentials = match sso::get_account_role_credentials(self, account_info, &role) {
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