use dkg_primitives::{Config, KeyGenerator, DbManager};
use radius_sdk::validation_service::{
    DkgValidation, DkgValidationServiceEvent, RestakingValidation,
};

#[derive(Debug)]
/// Worker that handles certain state stored on-chain
pub struct ValidationService<C: Config>(C);

impl<C: Config> ValidationService<C> {
    pub fn new(config: C) -> Self {
        Self(config)
    }

    async fn handle_task_created(&self) {
        tracing::info!("Got task created event! Responding to the task...");
        if let Ok(session_id) = self.0.db_manager().current_session() {
            let _ = self
            .0
            .validation_service()
            .respond_task(session_id.into())
            .await;
        }
    }

    // Set up the new operator list and trusted setup when the task response is received
    async fn handle_task_response(&self) {
        tracing::info!("Got task response event!");
    }

    async fn handle_task_completed(&self) {
        tracing::info!("Got task completed event!");
        if let Ok(new_committee_list) = self.0.validation_service().get_active_operators().await {
            tracing::info!("Updating next operator list");
            if let Err(e) = self.0.db_manager().update_next_operator_list(
                &new_committee_list
                    .into_iter()
                    .map(|o| o.into())
                    .collect::<Vec<_>>(),
            ) {
                // TODO: It is breaking issue. We need to handle it
                tracing::error!("Failed to update next operator list: {:?}", e);
            }
        }
        if let Ok(new_trusted_setup) = self.0.validation_service().get_active_trusted_setup().await
        {
            if let Ok(new_kg) = <<C as Config>::KeyGenerator as KeyGenerator>::setup(new_trusted_setup) {
                tracing::info!("Updating trusted setup");
                let kg = self.0.key_generator();
                let mut kg = kg.write().await;
                *kg = new_kg;
            } else {
                // TODO: Error Handling
            }
        }
    }

    pub async fn handle_callback_events(&self, event: DkgValidationServiceEvent) {
        match event {
            DkgValidationServiceEvent::TaskCreated(_) => self.handle_task_created().await,
            DkgValidationServiceEvent::TaskResponse(_) => self.handle_task_response().await,
            DkgValidationServiceEvent::TaskCompleted(_) => self.handle_task_completed().await,
        }
    }
}
