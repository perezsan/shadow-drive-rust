use anyhow::anyhow;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use solana_sdk::bs58;
use solana_sdk::signature::Signer;

const SIGNIN_MSG: &str = "Sign in to GenesysGo Shadow Platform.";
const PORTAL_SIGNIN_URL: &str = "https://portal.genesysgo.net/api/signin";
const RPC_SIGNIN_URL: &str = "https://portal.genesysgo.net/api/premium/token";

/// The response object for sign-in Step #1.
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesysGoAuthResponse {
    pub token: String,
    pub user: GenesysGoUser,
}

/// User data about the signed-in account.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesysGoUser {
    pub id: u64,
    pub public_key: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A token returned after completing sign-in Step #2.
/// This token can be used as a bearer token to make authenticated
/// RPC requests.
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

/// The request body for sign-in Step #1.
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesysGoAuth {
    message: String, // signed and base-58 encoded SIGNIN_MSG
    signer: String,
}

/// If you only need an bearer token for RPC and don't have or need to keep the portal token,
/// this is the top-level function to call. It performs both sign-in calls.
pub async fn authenticate(signer: &dyn Signer, account_id: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = genesysgo_portal_auth(signer, &client).await?;
    let resp = genesysgo_rpc_auth(account_id, &resp.token, &client).await?;
    Ok(resp.token)
}

/// First request, acquire a JWT needed for the second request.
pub async fn genesysgo_portal_auth(signer: &dyn Signer, client: &reqwest::Client) -> anyhow::Result<GenesysGoAuthResponse> {
    let signature = signer.sign_message(SIGNIN_MSG.as_bytes());
    let body = GenesysGoAuth {
        message: bs58::encode(signature.as_ref()).into_string(),
        signer: signer.pubkey().to_string(),
    };
    let resp = client
        .post(Url::parse(PORTAL_SIGNIN_URL)?)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body)?)
        .send()
        .await?;
    let auth_resp: GenesysGoAuthResponse = serde_json::from_str(&resp.text().await?)?;
    Ok(auth_resp)
}

/// Second request, uses the Bearer token from the first sign-in step,
/// and acquires JWT used to authenticate normal RPC requests.
pub async fn genesysgo_rpc_auth(
    account_id: &str,
    step_1_auth_token: &str,
    client: &reqwest::Client,
) -> anyhow::Result<TokenResponse> {
    let step2_url = RPC_SIGNIN_URL.to_owned() + "/" + account_id;
    let bearer_token = format!("Bearer {}", step_1_auth_token);
    let resp = client
        .post(Url::parse(&step2_url)?)
        .header("Content-Type", "application/json")
        .header("Authorization", &bearer_token)
        .send()
        .await?;
    let resp: TokenResponse = serde_json::from_str(&resp.text().await?)?;
    Ok(resp)
}

/// If you only have a GenesysGo RPC URL, this will parse out
/// the account ID necessary to perform a sign-in.
pub fn parse_account_id_from_url(genesysgo_url: String) -> anyhow::Result<String> {
    if !genesysgo_url.contains("genesysgo") {
        return Err(anyhow!("Not a genesysgo URL, cannot infer Account ID"));
    }
    let pieces = genesysgo_url.split("/");
    let last = pieces
        .last()
        .ok_or(anyhow!("Could not parse genesysgo url: {}", &genesysgo_url))?;
    Ok(last.to_string())
}
