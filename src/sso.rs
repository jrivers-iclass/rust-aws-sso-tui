use anyhow::Error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time;
use std::{fs, path::PathBuf, process::{Command, Stdio}};
use crate::{aws::{session_name, AccessToken, AccountInfo, AccountInfoProvider, SsoAccessTokenProvider}, App};
use aws_config::{BehaviorVersion, Region};
use directories::UserDirs;
use std::io::Write;

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
}

impl Default for ConfigProvider {
    fn default() -> Self {
        ConfigProvider {
            access_token: AccessToken::default(),
            account_info_provider: None,
        }
    }
}

#[::tokio::main]
pub async fn get_aws_config(start_url: &str, app: &mut App, new_token: Option<bool>) -> Result<ConfigProvider, anyhow::Error> {
    if start_url.is_empty() {
        return Err(Error::msg("SSO Start URL is required"));
    }
    let user_dirs = UserDirs::new().expect("Could not resolve user HOME.");
    let home_dir = user_dirs.home_dir();
    let aws_config_dir = home_dir.join(".aws");

    let config = aws_config::SdkConfig::builder()
        .region(Some(Region::new("us-east-1")))
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
            })
        }
        Err(e) => Err(e),
    }
}

#[::tokio::main]
pub async fn get_sso_accounts(config_provider: ConfigProvider) -> Result<Vec<AccountInfo>, anyhow::Error> {
    let mut sso_accounts = config_provider.account_info_provider.unwrap()
        .get_account_list(&config_provider.access_token)
        .await?;
    
    sso_accounts.sort();
    
    Ok(sso_accounts)
}

#[::tokio::main]
pub async fn get_account_roles(config_provider: ConfigProvider, account: AccountInfo) -> Result<Vec<String>, anyhow::Error> {
    time::sleep(time::Duration::from_millis(100)).await;    
    let roles = config_provider.account_info_provider.unwrap().get_roles_for_account(&config_provider.access_token, &account).await?;
    
    Ok(roles)
}

#[::tokio::main]
pub async fn get_account_role_credentials(config_provider: ConfigProvider, account: AccountInfo, role: &str) -> Result<RoleCredentials, anyhow::Error> {
    time::sleep(time::Duration::from_millis(100)).await;        
    // Get credentials for the role
    let role_credentials_output = config_provider.account_info_provider.unwrap().get_role_credentials(&config_provider.access_token, &account, role).await?;
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
    time::sleep(time::Duration::from_millis(100)).await;
    
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
    // open::with_command(federated_url, "firefox");   
    let profile_name = format!("aws-sso-{}-{}", account.account_id, role);
    let profile_dir = create_firefox_profile(&profile_name);
    let profile_path_str = profile_dir.to_str().unwrap();

    if cfg!(target_os = "windows") {
        // For Windows
        Command::new("powershell")
            .args(&["-Command", "Start-Process", "firefox", "-ArgumentList", &format!("'--new-instance', '--profile', '{}', '{}'", profile_path_str, &federated_url)])
            .status()
            .expect("failed to open browser");
    } else if cfg!(target_os = "macos") {
        // For macOS
        Command::new("open")
            .args(&["-na", "Firefox", "--args", "--new-instance", "--profile", profile_path_str, &federated_url])
            .status()
            .expect("failed to open browser");
    } else if cfg!(target_os = "linux") {
        // For Linux
        Command::new("firefox")
            .args(&["--new-instance", "--profile", profile_path_str, &federated_url])
            .status()
            .expect("failed to open browser");
    } else {
        // Fallback
        webbrowser::open(&federated_url).expect("failed to open browser");
    }

    Ok(())
}

pub fn export_env_vars(credentials: &RoleCredentials) -> Result<(), anyhow::Error> {
    if cfg!(target_os = "windows") {
        let env_vars = vec![
            format!("setx AWS_ACCESS_KEY_ID {}", &credentials.access_key_id),
            format!("setx AWS_SECRET_ACCESS_KEY {}", &credentials.secret_access_key),
            format!("setx AWS_SESSION_TOKEN {}", &credentials.session_token),
        ];  
        for env_var in env_vars {
            let _ = Command::new("cmd")
                .args(["/C", &env_var])
                .output();
        }
    } else {
        let env_vars = vec![
            format!("export AWS_ACCESS_KEY_ID={}", &credentials.access_key_id),
            format!("export AWS_SECRET_ACCESS_KEY={}", &credentials.secret_access_key),
            format!("export AWS_SESSION_TOKEN={}", &credentials.session_token),
        ];

        // let shell = if cfg!(target_os = "macos") { "zsh" } else { "bash" };
        // for env_var in env_vars {
        //     Command::new(shell)
        //         .args(["-c", &env_var])
        //         .output()
        //         .expect("Failed to execute command");
        // }

        let export_commands = env_vars.join("\n");
        let mut pbcopy = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start pbcopy");

        {
            let stdin = pbcopy.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(export_commands.as_bytes()).expect("Failed to write to pbcopy");
        }

        pbcopy.wait().expect("Failed to copy to clipboard");
    }
    Ok(())

}


fn create_firefox_profile(profile_name: &str) -> PathBuf {
    let user_dirs = UserDirs::new().expect("Could not find user directories");
    let profile_dir = user_dirs.home_dir().join(format!(".mozilla/firefox/{}.{}", profile_name, "aws-sso"));

    if !profile_dir.exists() {
        fs::create_dir_all(&profile_dir).expect("Could not create profile directory");

        // Create a basic prefs.js file for the profile
        let prefs_content = r#"
user_pref("browser.startup.homepage", "about:blank");
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("app.normandy.first_run", false);
        "#;
        fs::write(profile_dir.join("prefs.js"), prefs_content).expect("Could not write prefs.js file");
    }

    profile_dir
}