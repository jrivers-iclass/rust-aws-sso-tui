use std::{collections::HashMap, rc::Rc};

use ratatui::{layout::Rect, widgets::{
        ScrollbarState, TableState,
    }, Frame};

use crate::sso::{ConfigProvider, RoleCredentials};

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