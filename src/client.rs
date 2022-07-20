use std::time::Duration;

use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer, transaction::Transaction};

mod add_storage;
mod cancel_delete_storage_account;
mod claim_stake;
mod create_storage_account;
mod delete_file;
mod delete_storage_account;
mod edit_file;
mod get_storage_account;
mod list_objects;
mod make_storage_immutable;
mod reduce_storage;
mod store_files;
// mod upload_multiple_files;

pub use add_storage::*;
pub use cancel_delete_storage_account::*;
pub use claim_stake::*;
pub use create_storage_account::*;
pub use delete_file::*;
pub use delete_storage_account::*;
pub use edit_file::*;
pub use get_storage_account::*;
pub use list_objects::*;
pub use make_storage_immutable::*;
pub use reduce_storage::*;
pub use store_files::*;

use crate::{
    constants::SHDW_DRIVE_ENDPOINT,
    error::Error,
    models::{FileDataResponse, ShadowDriveResult, ShdwDriveResponse},
};

/// Client that allows a user to interact with the Shadow Drive.
pub struct ShadowDriveClient<T>
where
    T: Signer,
{
    wallet: T,
    rpc_client: RpcClient,
    http_client: reqwest::Client,
}

impl<T> ShadowDriveClient<T>
where
    T: Signer + Send + Sync,
{
    /// Creates a new [`ShadowDriveClient`] from the given [`Signer`] and URL.
    /// * `wallet` - A [`Signer`] that for signs all transactions generated by the client. Typically this is a user's keypair.
    /// * `rpc_url` - An HTTP URL of a Solana RPC provider.
    ///
    /// The underlying Solana RPC client is configured with 120s timeout and a [commitment level][cl] of [`Finalized`](solana_sdk::commitment_config::CommitmentLevel::Finalized).
    ///
    /// [cl]: https://docs.solana.com/developing/clients/jsonrpc-api#configuring-state-commitment
    ///
    /// To customize [`RpcClient`] settings see [`new_with_rpc`](Self::new_with_rpc).
    ///
    /// # Example
    /// ```
    /// use solana_sdk::signer::keypair::Keypair;    
    ///
    /// let wallet = Keypair::generate();
    /// let shdw_drive = ShadowDriveClient::new(wallet, "https://ssc-dao.genesysgo.net");
    /// ```
    pub fn new<U: ToString>(wallet: T, rpc_url: U) -> Self {
        let rpc_client = RpcClient::new_with_timeout_and_commitment(
            rpc_url.to_string(),
            Duration::from_secs(120),
            CommitmentConfig::finalized(),
        );
        Self {
            wallet,
            rpc_client,
            http_client: reqwest::Client::new(),
        }
    }

    /// Creates a new [`ShadowDriveClient`] from the given [`Signer`] and [`RpcClient`].
    /// * `wallet` - A [`Signer`] that for signs all transactions generated by the client. Typically this is a user's keypair.
    /// * `rpc_client` - A Solana [`RpcClient`] that handles sending transactions and reading accounts from the blockchain.
    ///
    /// Providng the [`RpcClient`] allows customization of timeout and committment level.
    ///
    /// # Example
    /// ```
    /// use solana_client::rpc_client::RpcClient;
    /// use solana_sdk::signer::keypair::Keypair;    
    /// use solana_sdk::commitment_config::CommitmentConfig;
    ///
    /// let wallet = Keypair::generate();
    /// let solana_rpc = RpcClient::new_with_commitment("https://ssc-dao.genesysgo.net", CommitmentConfig::confirmed());
    /// let shdw_drive = ShadowDriveClient::new_with_rpc(wallet, solana_rpc);
    /// ```
    pub fn new_with_rpc(wallet: T, rpc_client: RpcClient) -> Self {
        Self {
            wallet,
            rpc_client,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn get_object_data(&self, location: &str) -> ShadowDriveResult<FileDataResponse> {
        let response = self
            .http_client
            .post(format!("{}/get-object-data", SHDW_DRIVE_ENDPOINT))
            .header("Content-Type", "application/json")
            .json(&json!({ "location": location }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ShadowDriveServerError {
                status: response.status().as_u16(),
                message: response.json::<Value>().await?,
            });
        }

        let response = response.json::<FileDataResponse>().await?;

        Ok(response)
    }

    async fn send_shdw_txn(
        &self,
        uri: &str,
        txn_encoded: String,
    ) -> ShadowDriveResult<ShdwDriveResponse> {
        let body = serde_json::to_string(&json!({
           "transaction": txn_encoded,
           "commitment": "finalized"
        }))
        .map_err(Error::InvalidJson)?;

        let response = self
            .http_client
            .post(format!("{}/{}", SHDW_DRIVE_ENDPOINT, uri))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ShadowDriveServerError {
                status: response.status().as_u16(),
                message: response.json::<Value>().await?,
            });
        }

        let response = response.json::<ShdwDriveResponse>().await?;

        Ok(response)
    }
}

pub(crate) fn serialize_and_encode(txn: &Transaction) -> ShadowDriveResult<String> {
    let serialized = bincode::serialize(txn)
        .map_err(|error| Error::TransactionSerializationFailed(format!("{:?}", error)))?;
    Ok(base64::encode(serialized))
}
