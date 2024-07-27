pub fn session_name(start_url: &str) -> String {
    let start_url_without_schema = start_url.replace("https://", "");
    let (subdomain, _) = start_url_without_schema
        .split_once(".")
        .unwrap();

    format!("sso-{}", &subdomain)
}