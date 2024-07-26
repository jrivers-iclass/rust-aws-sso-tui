use reqwest::{Client, Request, Response};
use serde::{Deserialize, Serialize};
use tokio::time;
use std::process::Command;
use crate::aws::{session_name, AccountInfo, AccountInfoProvider, AwsCliConfigService, SsoAccessTokenProvider};
use aws_config::{BehaviorVersion, Region};
use directories::UserDirs;

#[derive(Default, Clone)]
pub struct RoleCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: String,
}

#[::tokio::main]
pub async fn get_sso_accounts() -> Result<Vec<AccountInfo>, anyhow::Error> {
    let start_url = "https://iclasspro.awsapps.com/start";
    let user_dirs = UserDirs::new().expect("Could not resolve user HOME.");
    let home_dir = user_dirs.home_dir();
    let aws_config_dir = home_dir.join(".aws");

    let config = aws_config::SdkConfig::builder()
        .region(Some(Region::new("us-east-1")))
        .behavior_version(BehaviorVersion::latest())
        .build();

    let account_info_provider = AccountInfoProvider::new(&config);
    let session_name = session_name(&start_url);
    let token_provider = SsoAccessTokenProvider::new(&config, session_name.as_str(), &aws_config_dir)?;
    let access_token = token_provider.get_access_token(&start_url).await?;

    let mut sso_accounts = account_info_provider
        .get_account_list(&access_token)
        .await?;
    
    sso_accounts.sort();
    
    Ok(sso_accounts)
}

#[::tokio::main]
pub async fn get_account_roles(account: AccountInfo) -> Result<Vec<String>, anyhow::Error> {
    time::sleep(time::Duration::from_millis(100)).await;
    let start_url = "https://iclasspro.awsapps.com/start";
    let user_dirs = UserDirs::new().expect("Could not resolve user HOME.");
    let home_dir = user_dirs.home_dir();
    let aws_config_dir = home_dir.join(".aws");

    let config = aws_config::SdkConfig::builder()
        .region(Some(Region::new("us-east-1")))
        .behavior_version(BehaviorVersion::latest())
        .build();

    let account_info_provider = AccountInfoProvider::new(&config);
    let session_name = session_name(&start_url);
    let token_provider = SsoAccessTokenProvider::new(&config, session_name.as_str(), &aws_config_dir)?;
    let access_token = token_provider.get_access_token(&start_url).await?;
    let roles = account_info_provider.get_roles_for_account(&access_token, &account).await?;
    
    Ok(roles)
}

#[::tokio::main]
pub async fn get_account_role_credentials(account: AccountInfo, role: &str) -> Result<(RoleCredentials), anyhow::Error> {
    time::sleep(time::Duration::from_millis(100)).await;
    let start_url = "https://iclasspro.awsapps.com/start";
    let user_dirs = UserDirs::new().expect("Could not resolve user HOME.");
    let home_dir = user_dirs.home_dir();
    let aws_config_dir = home_dir.join(".aws");

    let config = aws_config::SdkConfig::builder()
        .region(Some(Region::new("us-east-1")))
        .behavior_version(BehaviorVersion::latest())
        .build();

    let account_info_provider = AccountInfoProvider::new(&config);
    let session_name = session_name(&start_url);
    let token_provider = SsoAccessTokenProvider::new(&config, session_name.as_str(), &aws_config_dir)?;
    let access_token = token_provider.get_access_token(&start_url).await?;
    
    // Get credentials for the role
    let role_credentials_output = account_info_provider.get_role_credentials(&access_token, &account, role).await?;
    let role_credentials = role_credentials_output.role_credentials().unwrap();

    Ok( RoleCredentials {
        access_key_id: role_credentials.access_key_id().unwrap().to_string(),
        secret_access_key: role_credentials.secret_access_key().unwrap().to_string(),
        session_token: role_credentials.session_token().unwrap().to_string(),
        expiration: role_credentials.expiration().to_string(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionData {
    sessionId: String,
    sessionKey: String,
    sessionToken: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ContainerUrl {
    name: String,
    url: String
}

#[::tokio::main]
pub async fn open_console(account: AccountInfo, role: &str) -> Result<(), anyhow::Error> {
    time::sleep(time::Duration::from_millis(100)).await;
    let start_url = "https://iclasspro.awsapps.com/start";
    let user_dirs = UserDirs::new().expect("Could not resolve user HOME.");
    let home_dir = user_dirs.home_dir();
    let aws_config_dir = home_dir.join(".aws");

    let config = aws_config::SdkConfig::builder()
        .region(Some(Region::new("us-east-1")))
        .behavior_version(BehaviorVersion::latest())
        .build();

    let account_info_provider = AccountInfoProvider::new(&config);
    let session_name = session_name(&start_url);
    let token_provider = SsoAccessTokenProvider::new(&config, session_name.as_str(), &aws_config_dir)?;
    let access_token = token_provider.get_access_token(&start_url).await?;
    
    // Get credentials for the role
    let role_credentials_output = account_info_provider.get_role_credentials(&access_token, &account, role).await?;
    let role_credentials = role_credentials_output.role_credentials().unwrap();

    let session_data = SessionData {
        sessionId: role_credentials.access_key_id().unwrap().to_string(),
        sessionKey: role_credentials.secret_access_key().unwrap().to_string(),
        sessionToken: role_credentials.session_token().unwrap().to_string(),
    };

    let aws_federated_signin_endpoint = "https://signin.aws.amazon.com/federation";
    //let signin_token = format!("Action=getSigninToken&SessionType=json&Session={}", serde_json::to_string(&session_data)?);    
    let token_params = [
        ("Action", "getSigninToken"), 
        ("SessionDuration", "43200"),
        ("Session", &serde_json::to_string(&session_data)?)
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
    let _ = open::that(federated_url.clone());

    open::with_command("", format!("cmd /c set AWS_ACCESS_KEY_ID={}", &session_data.sessionId));
    let env_vars = vec![
        format!("setx AWS_ACCESS_KEY_ID {}", &session_data.sessionId),
        format!("setx AWS_SECRET_ACCESS_KEY {}", &session_data.sessionKey),
        format!("setx AWS_SESSION_TOKEN {}", &session_data.sessionToken),
    ];

    print!("Setting environment variables for AWS CLI...");


    for env_var in env_vars {
        print!("Setting environment variable: {}", env_var);
        let _ = Command::new("cmd")
            .args(["/C", &env_var])
            .output();
    }    

    Ok(())
}