use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Scope(String);

impl Scope {
    pub fn new(scope: impl AsRef<str>) -> Self {
        Self(scope.as_ref().to_owned())
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub scope: Vec<Scope>,
}

/// Authenticate using authorization code grant flow
/// <https://dev.twitch.tv/docs/authentication/getting-tokens-oauth#authorization-code-grant-flow>
pub async fn authenticate(
    client_id: &str,
    client_secret: &str,
    force_verify: bool,
    scopes: &[Scope],
) -> eyre::Result<Tokens> {
    // We will redirect user to this url to retrieve the token
    // This url should be same as specified in the twitch registered app
    let redirect_uri = "http://localhost:3000";

    // From twitch docs:
    // Although optional, you are strongly encouraged to pass a state string to
    // help prevent Cross-Site Request Forgery (CSRF) attacks. The server
    // returns this string to you in your redirect URI (see the state parameter
    // in the fragment portion of the URI). If this string doesn’t match the
    // state string that you passed, ignore the response. The state string
    // should be randomly generated and unique for each OAuth request.
    let state: String = rand::distributions::Alphanumeric
        .sample_iter(global_rng())
        .take(16)
        .map(|c| c as char)
        .collect();

    let mut authorize_url = Url::parse("https://id.twitch.tv/oauth2/authorize").unwrap();
    {
        // Set up query
        let mut query = authorize_url.query_pairs_mut();
        query.append_pair("client_id", client_id);
        query.append_pair("force_verify", &force_verify.to_string());
        query.append_pair("redirect_uri", redirect_uri);
        query.append_pair("response_type", "code");
        query.append_pair(
            "scope",
            &scopes
                .iter()
                .map(|scope| scope.as_str())
                .collect::<Vec<&str>>()
                .join(" "),
        );
        query.append_pair("state", &state);
    }

    info!("Opening {}", authorize_url);
    open::that(authorize_url.as_str())?;

    debug!("Waiting for the user to be redirected to {}", redirect_uri);
    let redirected_url = wait_for_request_uri().await?;
    let query: HashMap<_, _> = redirected_url.query_pairs().collect();

    if **query.get("state").expect("Expected to see state") != state {
        panic!("Hey, are you being hacked or something?");
    }
    if let Some(error) = query.get("error") {
        let description = query
            .get("error_description")
            .expect("Error without description");
        eyre::bail!("{error}: {description}");
    }
    let code: &str = query.get("code").expect("No code wat");

    debug!("Got the code, getting the token");
    let mut form = HashMap::new();
    form.insert("client_id", client_id);
    form.insert("client_secret", client_secret);
    form.insert("code", code);
    form.insert("grant_type", "authorization_code");
    form.insert("redirect_uri", redirect_uri);
    Ok(reqwest::Client::new()
        .post("https://id.twitch.tv/oauth2/token")
        .form(&form)
        .send()
        .await?
        .json()
        .await?)
}

// This test requires interacting with the browser, so run it directly:
//
// ```
// cargo test -- --ignored auth --nocapture
// ```
#[test]
#[ignore]
fn test_authenticate() {
    logger::init_for_tests();
    let secrets = secret::Config::read().unwrap();
    info!(
        "{:?}",
        block_on(authenticate(
            &secrets.ttv.client_id,
            &secrets.ttv.client_secret,
            true,
            &[Scope::new("channel:read:redemptions")],
        )),
    );
}

pub async fn refresh(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> eyre::Result<Tokens> {
    let mut form = HashMap::new();
    form.insert("client_id", client_id);
    form.insert("client_secret", client_secret);
    form.insert("grant_type", "refresh_token");
    form.insert("refresh_token", refresh_token);
    Ok(reqwest::Client::new()
        .post("https://id.twitch.tv/oauth2/token")
        .form(&form)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn validate(token: &str) -> eyre::Result<bool> {
    let response = reqwest::Client::new()
        .get("https://id.twitch.tv/oauth2/validate")
        .header("Authorization", format!("OAuth {}", token))
        .send()
        .await?;
    match response.status() {
        reqwest::StatusCode::OK => Ok(true),
        reqwest::StatusCode::UNAUTHORIZED => Ok(false),
        _ => eyre::bail!("Unexpected status {}", response.status()),
    }
}
