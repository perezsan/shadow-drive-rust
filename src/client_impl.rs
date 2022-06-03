use solana_client::rpc_client::RpcClient;
use solana_sdk::signer::Signer;

mod add_storage;
mod create_storage_account;
mod get_storage_account;
mod upload_file;
mod request_delete_storage_account;

pub use add_storage::*;
pub use create_storage_account::*;
pub use get_storage_account::*;
pub use upload_file::*;
pub use request_delete_storage_account::*;

pub struct Client<T>
where
    T: Signer,
{
    wallet: T,
    rpc_client: RpcClient,
    http_client: reqwest::Client,
}

impl<T> Client<T>
where
    T: Signer + Send + Sync,
{
    pub fn new(wallet: T, rpc_client: RpcClient) -> Self {
        Self {
            wallet,
            rpc_client,
            http_client: reqwest::Client::new(),
        }
    }
}
