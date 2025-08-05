use std::{fmt::Debug, str::FromStr};
use async_trait::async_trait;
use dkg_primitives::{OperatorServiceError, OperatorService, Parameter, Operator};
use alloy::{
    network::{Ethereum, EthereumWallet}, primitives::{Address as EthAddress, U256}, providers::{fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller}, 
    Identity, ProviderBuilder, RootProvider, WsConnect}, pubsub::PubSubFrontend, signers::local::LocalSigner, sol, sol_types::SolValue, transports::http::{reqwest::Url, Client, Http},
    rpc::types::Log,
};
use futures::{StreamExt, Stream};
use tokio::sync::mpsc;
use DkgBApp::{DkgBAppInstance, TaskCreated, TaskResponse, TaskCompleted};

type ContractStateProvider = DkgBAppInstance<
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

type ContractEventPubSub = DkgBAppInstance<PubSubFrontend, RootProvider<PubSubFrontend>>;

sol! {
    #![sol(rpc, all_derives)]
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
pub struct BAppService {
    pub state_provider: ContractStateProvider,
    pub contract_address: EthAddress,
    pub blockchain_event_tx: mpsc::Sender<Result<BAppEvent, OperatorServiceError>>,
    pub event_pubsub: Option<ContractEventPubSub>,
}

impl BAppService {
    pub fn new(http_url: &str, private_key: &str, contract_address: &str) -> (Self, mpsc::Receiver<Result<BAppEvent, OperatorServiceError>>) {
        let http_url = Url::parse(http_url).unwrap();
        let contract_address = contract_address.parse::<EthAddress>().unwrap();
        let signer = LocalSigner::from_str(private_key).unwrap();
        let wallet = EthereumWallet::new(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(http_url);
        let state_provider = DkgBApp::new(contract_address, provider);
        let (tx, rx) = mpsc::channel(1);
        let service = Self { state_provider, contract_address, blockchain_event_tx: tx, event_pubsub: None };
        (service, rx)
    }

    pub async fn subscribe_events(&mut self, blockchain_ws_rpc_url: &str) {
        self.with_event_pubsub(blockchain_ws_rpc_url).await;
        let task_created_event = self.event_pubsub.as_ref().expect("Event pubsub is not initialized").TaskCreated_filter().subscribe().await.map_err(|_| OperatorServiceError::AnyError("Failed to subscribe to task created event".to_string())).unwrap().into_stream();
        let task_response_event = self.event_pubsub.as_ref().expect("Event pubsub is not initialized").TaskResponse_filter().subscribe().await.map_err(|_| OperatorServiceError::AnyError("Failed to subscribe to task response event".to_string())).unwrap().into_stream();
        let task_completed_event = self.event_pubsub.as_ref().expect("Event pubsub is not initialized").TaskCompleted_filter().subscribe().await.map_err(|_| OperatorServiceError::AnyError("Failed to subscribe to task completed event".to_string())).unwrap().into_stream();
        let blockchain_event_tx = self.blockchain_event_tx.clone();
        tokio::spawn(spawn_event_handler(task_created_event, blockchain_event_tx.clone()));
        tokio::spawn(spawn_event_handler(task_response_event, blockchain_event_tx.clone()));
        tokio::spawn(spawn_event_handler(task_completed_event, blockchain_event_tx));
    }

    async fn with_event_pubsub(&mut self, blockchain_ws_rpc_url: &str) {
        let ws_provider = ProviderBuilder::new().on_ws(WsConnect::new(Url::parse(&blockchain_ws_rpc_url).unwrap())).await.expect("Failed to connect to WS");
        let event_pubsub = DkgBAppInstance::new(self.contract_address, ws_provider);
        self.event_pubsub = Some(event_pubsub);
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
pub enum BAppEvent {
    TaskCreated(TaskCreated),
    TaskResponse(TaskResponse),
    TaskCompleted(TaskCompleted),
}

macro_rules! impl_bapp_event_from {
    ($($event: ident), *) => {
        $(
            impl From<($event, Log)> for BAppEvent {
                fn from((event, _): ($event, Log)) -> Self {
                    BAppEvent::$event(event)
                }
            }
        )*
    }
}

impl_bapp_event_from!(TaskCreated, TaskResponse, TaskCompleted);

async fn spawn_event_handler<T, E>(
    mut event_stream: impl Stream<Item = Result<T, E>> + Unpin,
    blockchain_event_tx: mpsc::Sender<Result<BAppEvent, OperatorServiceError>>
) 
where
    T: Into<BAppEvent> + Debug + Clone
{
    while let Some(res) = event_stream.next().await {
        if let Ok(event) = res {
            if let Err(e) = blockchain_event_tx.send(Ok(event.clone().into())).await {
                tracing::error!("Failed to send {:?} event: {:?}", event, e);
            }
        }
    }
}

#[async_trait]
impl<Address> OperatorService<Address> for BAppService 
where
    Address: Parameter + From<Vec<u8>> + AsRef<[u8]>
{
    type TrustedSetup = DkgBApp::TrustedSetupParams;
    type Error = OperatorServiceError;

    async fn is_solver(&self, address: Address) -> Result<bool, Self::Error> {
        let res = self.state_provider.isSolver(convert_back(address).ok_or(OperatorServiceError::AnyError("Invalid address".to_string()))?).call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0)
    }
    async fn is_operator(&self, address: Address) -> Result<bool, Self::Error> {
        let res = self.state_provider.isActiveCommittee(convert_back(address).ok_or(OperatorServiceError::AnyError("Invalid address".to_string()))?).call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0)
    }
    async fn get_session_duration(&self) -> Result<u64, Self::Error> {
        let res = self.state_provider.getSessionDuration().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }
    async fn get_collecting_duration(&self) -> Result<u64, Self::Error> {
        let res = self.state_provider.getCollectingDuration().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }
    async fn get_threshold(&self) -> Result<u16, Self::Error> {
        let res = self.state_provider.getMinimumKeyThreshold().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }

    async fn update_trusted_setup<T>(&self, trusted_setup: T) -> Result<(), Self::Error> 
    where
        T: Into<Self::TrustedSetup> + Send + Sync + 'static
    {
        let trusted_setup: Self::TrustedSetup = trusted_setup.into();
        let bytes = trusted_setup.abi_encode();
        let _ = self.state_provider
            .updateActiveTrustedSetup(bytes.into())
            .gas(15_000_000)
            .gas_price(20000000000)
            .send()
            .await
            .map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn get_session_per_round(&self) -> Result<u64, Self::Error> {
        let res = self.state_provider.getSessionsPerRound().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.try_into().map_err(|_| OperatorServiceError::Overflow)?)
    }
    async fn get_active_trusted_setup<T>(&self) -> Result<T, Self::Error>
    where
        Self::TrustedSetup: Into<T>
    {
        let res = self.state_provider.getActiveTrustedSetup().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.into())
    }
    async fn get_solver_info(&self) -> Result<(Address, String, String), Self::Error> {
        let res = self.state_provider.getSolverInfo().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok((convert(res.currentSolver), res.clusterRpcUrl, res.externalRpcUrl))
    }
    async fn get_active_operators(&self) -> Result<Vec<Operator<Address>>, Self::Error> { 
        let res = self.state_provider.getActiveCommitteeList().call().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(res._0.into_iter().map(|info| Operator::new(convert(info.account), info.clusterRpcUrl, info.externalRpcUrl)).collect())
    }
    async fn is_ready(&self, threshold: u16) -> Result<bool, Self::Error> {
        let res: Vec<Operator<Address>> = self.get_active_operators().await?;
        Ok(res.len() >= threshold as usize)
    }
    async fn create_new_task(&self, data: Vec<u8>) -> Result<(), Self::Error> {
        let _ = self.state_provider.createNewTask(data.into()).send().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn respond_to_task(&self, round: u64) -> Result<(), Self::Error> {
        let _ = self.state_provider.respondToTask(U256::from(round)).send().await.map_err(|e| OperatorServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
}
