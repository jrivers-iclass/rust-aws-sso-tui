use anyhow::Error;
use ini::Ini;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, process::Command};
use crate::{aws::{session_name, AccessToken, AccountInfo, AccountInfoProvider, SsoAccessTokenProvider}, App, ConfigOption};
use aws_config::{BehaviorVersion, Region};
use directories::UserDirs;
use urlencoding::encode;

#[derive(Default, Clone)]
pub struct RoleCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: String,
}

#[derive(Clone)]
pub struct ConfigProvider {
    pub access_token: AccessToken,
    pub account_info_provider: Option<AccountInfoProvider>,
    pub token_provider: Option<SsoAccessTokenProvider>,
}

impl Default for ConfigProvider {
    fn default() -> Self {
        ConfigProvider {
            access_token: AccessToken::default(),
            account_info_provider: None,
            token_provider: None,
        }
    }
}

#[::tokio::main]
pub async fn get_aws_config(start_url: &str, region: &str, app: &mut App, new_token: Option<bool>) -> Result<ConfigProvider, anyhow::Error> {
    if start_url.is_empty() {
        return Err(Error::msg("SSO Start URL is required"));
    }
    let user_dirs = UserDirs::new().expect("Could not resolve user HOME.");
    let home_dir = user_dirs.home_dir();
    let aws_config_dir = home_dir.join(".aws");

    let config = aws_config::SdkConfig::builder()
        .region(Some(Region::new(region.to_string())))
        .behavior_version(BehaviorVersion::latest())
        .build();

    let session_name = session_name(&start_url);
    let token_provider = SsoAccessTokenProvider::new(&config, session_name.as_str(), &aws_config_dir)?;
    let access_token = token_provider.get_access_token(&start_url, new_token.unwrap_or(false), app).await;

    match access_token {
        Ok(token) => {
            Ok(ConfigProvider {
                access_token: token,
                account_info_provider: Some(AccountInfoProvider::new(&config)),
                token_provider: Some(token_provider),
            })
        }
        Err(e) => Err(e),
    }
}

#[::tokio::main]
pub async fn get_sso_accounts(app: &mut App) -> Result<Vec<AccountInfo>, anyhow::Error> {
    let config_provider = app.aws_config_provider.clone();
    let token_provider = &config_provider.token_provider.as_ref().unwrap();
    let start_url = &app.config_options.options.iter().find(|option| option.name == "start_url").unwrap().value.clone();
    let access_token = token_provider.get_access_token(start_url, false, app).await?;

    let mut sso_accounts = config_provider.account_info_provider.as_ref().unwrap()
        .get_account_list(&access_token)
        .await?;
    
    sso_accounts.sort();
    
    Ok(sso_accounts)
}

#[::tokio::main]
pub async fn get_account_roles(app: &mut App, account: AccountInfo) -> Result<Vec<String>, anyhow::Error> {
    let config_provider = app.aws_config_provider.clone();
    let token_provider = &config_provider.token_provider.as_ref().unwrap();
    let start_url = &app.config_options.options.iter().find(|option| option.name == "start_url").unwrap().value.clone();
    let access_token = token_provider.get_access_token(start_url, false, app).await?;

    let roles = config_provider.account_info_provider.unwrap().get_roles_for_account(&access_token, &account).await?;
    
    Ok(roles)
}

#[::tokio::main]
pub async fn get_account_role_credentials(app: &mut App, account: AccountInfo, role: &str) -> Result<RoleCredentials, anyhow::Error> {     
    let config_provider = app.aws_config_provider.clone();
    let token_provider = &config_provider.token_provider.as_ref().unwrap();
    let start_url = &app.config_options.options.iter().find(|option| option.name == "start_url").unwrap().value.clone();
    let access_token = token_provider.get_access_token(start_url, false, app).await?;

    // Get credentials for the role
    let role_credentials_output = config_provider.account_info_provider.unwrap().get_role_credentials(&access_token, &account, role).await?;
    let role_credentials = role_credentials_output.role_credentials().unwrap();

    Ok( RoleCredentials {
        access_key_id: role_credentials.access_key_id().unwrap().to_string(),
        secret_access_key: role_credentials.secret_access_key().unwrap().to_string(),
        session_token: role_credentials.session_token().unwrap().to_string(),
        expiration: role_credentials.expiration().to_string(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionData {
    session_id: String,
    session_key: String,
    session_token: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ContainerUrl {
    name: String,
    url: String
}

#[::tokio::main]
pub async fn open_console(role_credentials: RoleCredentials, account: AccountInfo, role: &str) -> Result<(), anyhow::Error> {
    let session_data = SessionData {
        session_id: role_credentials.access_key_id.to_string(),
        session_key: role_credentials.secret_access_key.to_string(),
        session_token: role_credentials.session_token.to_string(),
    };

    let aws_federated_signin_endpoint = "https://signin.aws.amazon.com/federation";
    let session_data_json = serde_json::to_string(&session_data)?;


    let token_params = [
        ("Action", "getSigninToken"), 
        ("SessionDuration", "43200"),
        ("Session", &session_data_json)
    ];

    let client = Client::new();
    let response = client.get(aws_federated_signin_endpoint)
        .query(&token_params)
        .send()
        .await?;

    let signin_token_resonse = response.text().await?;
    let binding = serde_json::from_str::<serde_json::Value>(&signin_token_resonse)?;
    let signin_token = binding.get("SigninToken").unwrap().as_str().unwrap();

    let federated_params = [
        ("Action", "login"), 
        ("Issuer", ""),
        ("Destination", "https://console.aws.amazon.com/"), 
        ("SigninToken", &signin_token)
    ];

    let federated_url = format!("{}?{}", aws_federated_signin_endpoint, serde_urlencoded::to_string(&federated_params)?);     
    let profile_name = format!("aws-sso-{}-{}", account.account_id, role);

    let granted_container_url = ContainerUrl {
        name: profile_name.to_string(),
        url: encode(&federated_url).to_string()
    };

    let granted_container_oss = format!("ext+granted-containers:name={}&url={}", granted_container_url.name, granted_container_url.url);

    if cfg!(target_os = "windows") {
        // For Windows
        Command::new("powershell")
            .args(&["-Command", "Start-Process", "firefox", "-ArgumentList", &format!("'--new-tab', '{}'", &granted_container_oss)])
            .status()
            .expect("failed to open browser");
    } else if cfg!(target_os = "macos") {
        // For macOS
        Command::new("open")
            .args(&["-na", "Firefox", "--args", "--new-tab",  &granted_container_oss])
            .status()
            .expect("failed to open browser");
    } else if cfg!(target_os = "linux") {
        // For Linux
        Command::new("firefox")
            .args(&["--new-table", &granted_container_oss])
            .status()
            .expect("failed to open browser");
    } else {
        // Fallback
        webbrowser::open(&federated_url).expect("failed to open browser");
    }

    Ok(())
}

pub fn get_default_aws_path() -> PathBuf {
    let user_dirs = UserDirs::new().expect("Could not find user directories");
    let home_dir = user_dirs.home_dir();
    let aws_config_dir = home_dir.join(".aws");

    aws_config_dir
}

pub fn export_env_vars(credentials: &RoleCredentials, aws_config_path: ConfigOption) -> Result<(), anyhow::Error> {
    let file_path =&PathBuf::from(&aws_config_path.value).join("credentials");
    
    let mut config = Ini::new();
    if !file_path.exists() {
        let _ = std::fs::create_dir_all(file_path.parent().unwrap());
        let _ = std::fs::write(file_path, "".as_bytes());        
    }

    config.with_section(Some("default".to_string()))
            .set("AWS_ACCESS_KEY_ID", &credentials.access_key_id)
            .set("AWS_SECRET_ACCESS_KEY", &credentials.secret_access_key)
            .set("AWS_SESSION_TOKEN", &credentials.session_token);

    let _ = config.write_to_file(file_path.clone());
        
    Ok(())

}

// fn check_for_granted_extension() -> bool {
//     unimplemented!()
// }