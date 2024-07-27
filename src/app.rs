use ratatui::widgets::{
        ScrollbarState, TableState,
    };

use crate::sso::RoleCredentials;

#[derive(Default, Clone)]
pub struct AccountRow {
    pub account_name: String,
    pub account_id: String,
    pub roles: Vec<String>,
}

#[derive(Default, Clone)]
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
}