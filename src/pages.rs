pub mod accounts;
pub mod credentials;
pub mod roles;
pub mod config;
mod page;

pub use accounts::AccountsPage;
pub use credentials::CredentialsPage;
pub use roles::RolesPage;
pub use config::ConfigPage;
pub use page::*;