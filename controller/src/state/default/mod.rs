use crate::managers::{
    agent::{AgentManagerError, AgentManagerTrait},
    runtime::{FeatType, RuntimeError, RuntimeInfoStorage, RuntimeManager},
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum DefaultError {
    #[error("agent process failed: {0}")]
    AgentError(#[from] AgentManagerError),
    #[error("failed to get runtime info: {0}")]
    RuntimeError(#[from] RuntimeError),
}

pub struct DefaultState<'a, A, H>
where
    A: AgentManagerTrait,
    H: RuntimeInfoStorage,
{
    agent_manager: &'a Arc<Mutex<A>>,
    runtime_manager: &'a RuntimeManager<H>,
}

impl<'a, A, H> DefaultState<'a, A, H>
where
    A: AgentManagerTrait,
    H: RuntimeInfoStorage,
{
    pub fn new(agent_manager: &'a Arc<Mutex<A>>, runtime_manager: &'a RuntimeManager<H>) -> Self {
        DefaultState {
            agent_manager,
            runtime_manager,
        }
    }

    pub async fn execute(&self) -> Result<(), DefaultError> {
        let mut agent_processes = self.runtime_manager.filter_process_infos(FeatType::Agent)?;
        agent_processes.retain(|agent_process| {
            self.runtime_manager
                .is_running_or_remove_if_stopped(agent_process)
        });
        if agent_processes.len() > 1 {
            log::error!("Agent already running");
            return Ok(());
        }

        #[cfg(unix)]
        {
            let agent_manager = self.agent_manager.lock().await;
            let process_info = agent_manager.launch_agent()?;
            self.runtime_manager.add_process_info(process_info)?;
        }

        Ok(())
    }
}
