use ratatui::widgets::{
        ScrollbarState, TableState,
    };

use crate::sso::{ConfigProvider, RoleCredentials};

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
        }
    }
}