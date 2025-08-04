use std::str::FromStr;
use async_trait::async_trait;
use dkg_primitives::{OperatorServiceError, OperatorService, Parameter, Operator};
use alloy::{
    network::{Ethereum, EthereumWallet}, primitives::{Address as EthAddress, U256}, 
    providers::{fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller}, 
    Identity, ProviderBuilder, RootProvider}, 
    signers::local::LocalSigner, sol, sol_types::SolValue, transports::http::{reqwest::Url, Client, Http}
};
use futures::{StreamExt, Stream};
use tokio::sync::mpsc;

type ContractInstance = DkgBApp::DkgBAppInstance<
    Http<Client>,
    FillProvider<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<Http<Client>>,
        Http<Client>,
        Ethereum,
    >,
>;

sol! {
    #[sol(rpc)]
    DkgBApp,
    "src/ssv/DkgBApp.json"
}

impl From<DkgBApp::TrustedSetupParams> for dkg_node_skde_key_generator::SkdeParams {
    fn from(value: DkgBApp::TrustedSetupParams) -> Self {
        Self {
            n: value.n,
            g: value.g,
            t: value.t,
            h: value.h,
            max_sequencer_number: value.max_sequencer_number,
        }
    }
}

impl Into<DkgBApp::TrustedSetupParams> for dkg_node_skde_key_generator::SkdeParams {
    fn into(self) -> DkgBApp::TrustedSetupParams {
        DkgBApp::TrustedSetupParams {
            n: self.n,
            g: self.g,
            t: self.t,
            h: self.h,
            max_sequencer_number: self.max_sequencer_number,
        }
    }
}

#[derive(Debug, Clone)]
/// Client that interacts with the blockchain
pub struct BlockchainOperatorService {
    pub contract: ContractInstance,
    pub blockchain_event_tx: mpsc::Sender<Result<BlockchainEvent, OperatorServiceError>>,
}

impl BlockchainOperatorService {
    pub fn new(endpoint: &str, private_key: &str, contract_address: &str) -> (Self, mpsc::Receiver<Result<BlockchainEvent, OperatorServiceError>>) {
        let url = Url::parse(endpoint).unwrap();
        let signer = LocalSigner::from_str(private_key).unwrap();
        let wallet = EthereumWallet::new(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(url);
        let contract = DkgBApp::new(contract_address.parse::<EthAddress>().unwrap(), provider);
        let (tx, rx) = mpsc::channel(1);
        let service = Self { contract, blockchain_event_tx: tx };
        (service, rx)
    }
}

fn convert<Address>(address: EthAddress) -> Address 
where
    Address: From<Vec<u8>>
{
    let address = address.0.to_vec();
    address.into()
}

fn convert_back<Address>(address: Address) -> Option<EthAddress> 
where
    Address: AsRef<[u8]>,
{
    let address = address.as_ref().to_vec();
    if address.len() != 20 {
        return None;
    }
    let eth_address = EthAddress::from_slice(&address);
    Some(eth_address)
}

#[derive(Debug, Clone)]
pub enum BlockchainEvent {
    TaskCreated,
    TaskResponse,
}

async fn spawn_event_handler<T, E>(
    mut event_stream: impl Stream<Item = Result<T, E>> + Unpin,
    blockchain_event: BlockchainEvent,
    blockchain_event_tx: mpsc::Sender<Result<BlockchainEvent, OperatorServiceError>>
) {
    while let Some(res) = event_stream.next().await {
        if let Ok(_) = res {
            if let Err(e) = blockchain_event_tx.send(Ok(blockchain_event.clone())).await {
                tracing::error!("Failed to send {:?} event: {:?}", blockchain_event, e);
            }
        }
    }
}

impl BlockchainOperatorService {
    pub async fn subscribe_events(&self) {
        let task_created_event = self.contract.TaskCreated_filter().subscribe().await.map_err(|_| OperatorServiceError::AnyError("Failed to subscribe to task created event".to_string())).unwrap().into_stream();
        let task_response_event = self.contract.TaskResponse_filter().subscribe().await.map_err(|_| OperatorServiceError::AnyError("Failed to subscribe to task response event".to_string())).unwrap().into_stream();
        let blockchain_event_tx = self.blockchain_event_tx.clone();
        tokio::spawn(spawn_event_handler(task_created_event, BlockchainEvent::TaskCreated, blockchain_event_tx.clone()));
        tokio::spawn(spawn_event_handler(task_response_event, BlockchainEvent::TaskResponse, blockchain_event_tx));
    }
}

#[async_trait]
impl<Address> OperatorService<Address> for BlockchainOperatorService 
where
    Address: Parameter + From<Vec<u8>> + AsRef<[u8]>
{
    type TrustedSetup = DkgBApp::TrustedSetupParams;
    type Error = OperatorServiceError;

    async fn is_solver(&self, address: Address) -> Result<bool, Self::Error> {
        let res = self.contract.isSolver(convert_back(address).ok_or(OperatorServiceError::AnyError("Invalid address".to_string()))?).call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0)
    }
    async fn is_operator(&self, address: Address) -> Result<bool, Self::Error> {
        let res = self.contract.isActiveCommittee(convert_back(address).ok_or(OperatorServiceError::AnyError("Invalid address".to_string()))?).call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0)
    }
    async fn get_session_duration(&self) -> Result<u64, Self::Error> {
        let res = self.contract.getSessionDuration().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }
    async fn get_collecting_duration(&self) -> Result<u64, Self::Error> {
        let res = self.contract.getCollectingDuration().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }
    async fn get_threshold(&self) -> Result<u16, Self::Error> {
        let res = self.contract.getMinimumKeyThreshold().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }

    async fn update_trusted_setup<T>(&self, trusted_setup: T) -> Result<(), Self::Error> 
    where
        T: Into<Self::TrustedSetup> + Send + Sync + 'static
    {
        let trusted_setup: Self::TrustedSetup = trusted_setup.into();
        let bytes = trusted_setup.abi_encode();
        let _ = self.contract
            .updateActiveTrustedSetup(bytes.into())
            .gas(15_000_000)
            .gas_price(20000000000)
            .send()
            .await
            .map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn get_session_per_round(&self) -> Result<u64, Self::Error> {
        let res = self.contract.getSessionsPerRound().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }
    async fn get_active_trusted_setup<T>(&self) -> Result<T, Self::Error>
    where
        Self::TrustedSetup: Into<T>
    {
        let res = self.contract.getActiveTrustedSetup().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.into())
    }
    async fn get_solver_info(&self) -> Result<(Address, String, String), Self::Error> {
        let res = self.contract.getSolverInfo().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok((convert(res.currentSolver), res.clusterRpcUrl, res.externalRpcUrl))
    }
    async fn get_active_operators(&self) -> Result<Vec<Operator<Address>>, Self::Error> { 
        let res = self.contract.getActiveCommitteeList().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.into_iter().map(|info| Operator::new(convert(info.account), info.clusterRpcUrl, info.externalRpcUrl)).collect())
    }
    async fn is_ready(&self, threshold: u16) -> Result<bool, Self::Error> {
        let res: Vec<Operator<Address>> = self.get_active_operators().await?;
        Ok(res.len() >= threshold as usize)
    }
    async fn create_new_task(&self, data: Vec<u8>) -> Result<(), Self::Error> {
        let _ = self.contract.createNewTask(data.into()).send().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn respond_to_task(&self, round: u64) -> Result<(), Self::Error> {
        let _ = self.contract.respondToTask(U256::from(round)).send().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
}
