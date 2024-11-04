use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::process::agent::AgentEventListener;

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub state: State,
    pub agent_infos: Vec<AgentInfo>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum State {
    Default,
    Updating,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentInfo {
    pub process_id: u32,
    pub executed_at: DateTime<FixedOffset>,
    pub version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Failed to open or access file: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Failed to parse JSON content: {0}")]
    JsonParseError(#[from] serde_json::Error),
}

impl RuntimeInfo {
    pub fn default() -> Self {
        RuntimeInfo {
            state: State::Default,
            agent_infos: vec![],
        }
    }

    pub fn load_or_default(path: &PathBuf, lock: Arc<Mutex<()>>) -> Self {
        let _guard = lock.lock().unwrap();
        Self::read(path).unwrap_or_else(|_| Self::default())
    }

    pub fn read(path: &PathBuf) -> Result<Self, RuntimeError> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let runtime_info = serde_json::from_str(&content)?;
        Ok(runtime_info)
    }

    pub fn add_agent_info(&mut self, agent_info: AgentInfo) {
        println!("Adding agent info: {}", agent_info.process_id);
        self.agent_infos.push(agent_info);
    }

    pub fn write(&self, path: &PathBuf) -> Result<(), RuntimeError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let json_data = serde_json::to_string(self)?;
        file.write_all(json_data.as_bytes())?;
        Ok(())
    }

    pub fn remove_agent_info(&mut self, process_id: u32) {
        self.agent_infos
            .retain(|agent| agent.process_id != process_id);
    }

    // pub fn terminate_all_agents(&mut self) {
    //     for agent_info in &self.agent_infos {
    //         agent_info.terminate();
    //     }
    //     self.agent_infos.clear();
    // }
}

impl AgentEventListener for RuntimeInfo {
    fn on_agent_started(&mut self, agent_info: AgentInfo) {
        println!("Agent started with PID: {}", agent_info.process_id);
        self.add_agent_info(agent_info);
    }

    fn on_agent_terminated(&mut self, process_id: u32) {
        println!("Agent terminated with PID: {}", process_id);
        self.remove_agent_info(process_id);
    }
}
