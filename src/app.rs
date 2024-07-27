use std::{collections::HashMap, rc::Rc};
use crate::{aws::AccountInfo, sso, tui, widgets};
use directories::UserDirs;
use ini::Ini;
use ratatui::{
    layout::Rect, widgets::{
        ScrollbarState, TableState,
    }, 
    crossterm::event::{self, Event, KeyEvent, KeyEventKind},
    Frame
};
use color_eyre::{
    eyre::{Error, WrapErr},
    Result,
};
use crate::sso::{ConfigProvider, RoleCredentials};

const ITEM_HEIGHT: usize = 4;

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum CurrentPage{
    AccountList,
    Config,
    Credentials,
    Roles
}

#[derive(Clone)]
pub struct RouteConfig {
    pub layout: fn(&mut Frame) -> Rc<[Rect]>,
    pub render: fn(&mut Frame, &mut App, Rect),
}

#[derive(Default, Clone)]
pub struct AccountRow {
    pub account_name: String,
    pub account_id: String,
    pub roles: Vec<String>,
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
    pub start_url: String,
    pub value_input: String,
    pub currently_editing: bool,
    pub token_prompt: String,
    pub current_page: CurrentPage,
    pub routes: HashMap<CurrentPage, RouteConfig>,
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
            start_url: String::new(),
            value_input: String::new(),
            currently_editing: false,
            token_prompt: String::new(),
            current_page: CurrentPage::AccountList,
            routes: HashMap::new(),
        }
    }
}

impl App {    
    pub fn load_config(&mut self) -> Result<Ini, Error> {
        let file_path = UserDirs::new().unwrap().home_dir().join(".rust-tui").join("config.ini");
    
        let mut config = Ini::new();
        if !file_path.exists() {
            let _ = std::fs::create_dir_all(file_path.parent().unwrap());
            let _ = std::fs::write(file_path.clone(), "".as_bytes());
    
            config.with_section(Some("Main".to_string()))
                .set("start_url", "");
    
            self.update_config(&mut config)?;
        } else {
            config = Ini::load_from_file(file_path.clone())?;
        }
    
        Ok(config)
    }
    
    pub fn update_config(&mut self, config: &mut Ini) -> Result<(), Error> {
        let file_path = UserDirs::new().unwrap().home_dir().join(".rust-tui").join("config.ini");
        config.write_to_file(file_path)?;
        Ok(())
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {   
        self.routes = self.create_routes();
        let config = self.load_config()?;    

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

    pub fn load_aws_config(&mut self, new_token: Option<bool>) {
        self.aws_config_provider = match sso::get_aws_config(self.start_url.clone().as_str(), self, Some(new_token.unwrap_or(false))) {
            Ok(access_token) => access_token,
            Err(_) => ConfigProvider::default(),
        };
    }

    pub fn get_account_list(&mut self) {
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
        if self.currently_editing {
            self.route(frame, CurrentPage::Config);
        } else if self.role_is_selected {
            self.route(frame,CurrentPage::Credentials);
        } else if self.is_selected {
            self.route(frame,CurrentPage::Roles);
        } else {
            self.route(frame,CurrentPage::AccountList);
        }
    }

    fn route(&mut self, frame: &mut Frame, page: CurrentPage) {
        if let Some(route) = self.routes.get(&page) {
            let rects = (route.layout)(frame);
            (route.render)(frame, self, rects[0]);
            self.current_page = page;
        }
    }

    fn create_routes(&mut self) -> HashMap<CurrentPage, RouteConfig> {
        let mut routes = HashMap::new();

        // Config route
        routes.insert(CurrentPage::Config, RouteConfig {
            layout: |frame| widgets::config::get_layout(frame),
            render: |frame, mut app, rect| widgets::render_config(frame, &mut app, rect),
        });

        // Credentials route
        routes.insert(CurrentPage::Credentials, RouteConfig {
            layout: |frame| widgets::credentials::get_layout(frame),
            render: |frame, mut app, rect| widgets::render_credentials(frame, &mut app, rect),
        });

        // AccountList route
        routes.insert(CurrentPage::AccountList, RouteConfig {
            layout: |frame| widgets::accounts::get_layout(frame),
            render: |frame, mut app, rect| widgets::render_accounts(frame, &mut app, rect),
        });

        // Roles route
        routes.insert(CurrentPage::Roles, RouteConfig {
            layout: |frame| widgets::roles::get_layout(frame),
            render: |frame, mut app, rect| {
                widgets::render_accounts(frame, &mut app, rect);
                if app.is_selected {
                    let rects = widgets::roles::get_layout(frame);
                    widgets::render_roles(frame, &mut app, rects[1]);
                }
            },
        });

        routes
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

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        self.credential_message = "".to_string();
        match self.current_page {
            CurrentPage::AccountList => {
                let _ = widgets::accounts::handle_key_events(self, key_event);
            }
            CurrentPage::Roles => {
                let _ = widgets::roles::handle_key_events(self, key_event);
            }
            CurrentPage::Credentials => {
                let _ = widgets::credentials::handle_key_events(self, key_event);
            }
            CurrentPage::Config => {
                let _ = widgets::config::handle_key_events(self, key_event);
            }
        }

        Ok(())
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