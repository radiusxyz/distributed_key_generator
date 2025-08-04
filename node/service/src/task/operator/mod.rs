use dkg_primitives::{Config, KeyGenerator, OperatorService, OperatorTrustedSetupFor, TrustedSetupFor};
use dkg_node_primitives::OperatorServiceError;
use dkg_node_operator::BlockchainEvent;
use dkg_rpc::DbManager;
use tokio::sync::mpsc;

pub async fn start_blockchain_operator_worker<C: Config>(config: &C, blockchain_event_rx: mpsc::Receiver<Result<BlockchainEvent, OperatorServiceError>>) -> Result<tokio::task::JoinHandle<()>, C::Error>
where
    TrustedSetupFor<C>: From<OperatorTrustedSetupFor<C>>, 
{
    let mut worker = BlockchainOperatorWorker::<C>::new(config.clone(), blockchain_event_rx);
    let handle = tokio::spawn(async move { worker.run().await });
    Ok(handle)
}

#[derive(Debug)]
/// Worker that handles certain state stored on-chain
pub struct BlockchainOperatorWorker<C: Config> {
    pub config: C,
    pub blockchain_event_rx: mpsc::Receiver<Result<BlockchainEvent, OperatorServiceError>>,
}

impl<C: Config> BlockchainOperatorWorker<C> 
where
    TrustedSetupFor<C>: From<OperatorTrustedSetupFor<C>>,
{
    pub fn new(config: C, blockchain_event_rx: mpsc::Receiver<Result<BlockchainEvent, OperatorServiceError>>) -> Self {
        Self { config, blockchain_event_rx }
    }

    pub async fn handle_task_created(&mut self) {
        tracing::info!("Got task created event!");
    }

    // Set up the new operator list and trusted setup when the task response is received
    pub async fn handle_task_response(&mut self) {
        if let Ok(new_committee_list) = self.config.operator_service().get_active_operators().await {
            if let Err(e) = self.config.db_manager().update_next_operator_list(new_committee_list) {
                // TODO: It is breaking issue. We need to handle it
                tracing::error!("Failed to update next operator list: {:?}", e);
            }
        }
        if let Ok(new_trusted_setup) = self.config.operator_service().get_active_trusted_setup().await {
            if let Err(e) = self.config.key_generator_mut().update_trusted_setup(new_trusted_setup) {
                // TODO: It is breaking issue. We need to handle it
                tracing::error!("Failed to update trusted setup: {:?}", e);
            }
        }
        tracing::info!("Got task response event!");
    }

    pub async fn subscribe_events(&mut self) {
        loop {
            while let Some(event) = self.blockchain_event_rx.recv().await {
                match event {
                    Ok(BlockchainEvent::TaskCreated) => {
                        self.handle_task_created().await;
                    }, 
                    Ok(BlockchainEvent::TaskResponse) => {
                        self.handle_task_response().await;
                    }
                    Err(e) => {
                        tracing::error!("Failed to receive blockchain event: {:?}", e);
                        continue;
                    }
                }
            }
        }
    }

    /// Simple loop that runs forever
    pub async fn run(&mut self) {
        self.subscribe_events().await
    }
}