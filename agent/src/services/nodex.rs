use crate::nodex::extension::secure_keystore::FileBaseKeyStore;
use crate::nodex::keyring;
use crate::nodex::utils::sidetree_client::SideTreeClient;
use crate::{app_config, server_config};
use anyhow;
use bytes::Bytes;
#[cfg(unix)]
use controller::process::runtime::{FeatType, RuntimeInfo, State};
#[cfg(unix)]
use controller::process::systemd::{is_manage_by_systemd, is_manage_socket_activation};
#[cfg(unix)]
use controller::state::resource::ResourceManager;
#[cfg(unix)]
use daemonize::Daemonize;
use fs2::FileExt;
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use protocol::did::did_repository::{DidRepository, DidRepositoryImpl};
use protocol::did::sidetree::payload::DidResolutionResponse;
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::Command,
};
use zip::ZipArchive;

pub struct NodeX {
    did_repository: DidRepositoryImpl<SideTreeClient>,
}

impl NodeX {
    pub fn new() -> Self {
        let server_config = server_config();
        let sidetree_client = SideTreeClient::new(&server_config.did_http_endpoint()).unwrap();
        let did_repository = DidRepositoryImpl::new(sidetree_client);

        NodeX { did_repository }
    }

    pub fn did_repository(&self) -> &DidRepositoryImpl<SideTreeClient> {
        &self.did_repository
    }

    pub async fn create_identifier(&self) -> anyhow::Result<DidResolutionResponse> {
        // NOTE: find did
        let config = app_config();
        let keystore = FileBaseKeyStore::new(config.clone());
        if let Some(did) =
            keyring::keypair::KeyPairingWithConfig::load_keyring(config.clone(), keystore.clone())
                .ok()
                .and_then(|v| v.get_identifier().ok())
        {
            if let Some(json) = self.find_identifier(&did).await? {
                return Ok(json);
            }
        }

        let mut keyring_with_config =
            keyring::keypair::KeyPairingWithConfig::create_keyring(config, keystore);
        let res = self
            .did_repository
            .create_identifier(keyring_with_config.get_keyring())
            .await?;
        keyring_with_config.save(&res.did_document.id);

        Ok(res)
    }

    pub async fn find_identifier(
        &self,
        did: &str,
    ) -> anyhow::Result<Option<DidResolutionResponse>> {
        let res = self.did_repository.find_identifier(did).await?;

        Ok(res)
    }

    pub async fn update_version(
        &self,
        binary_url: &str,
        output_path: PathBuf,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            binary_url.starts_with("https://github.com/nodecross/nodex/releases/download/"),
            "Invalid url"
        );

        #[cfg(unix)]
        let agent_filename = { "nodex-agent" };
        #[cfg(windows)]
        let agent_filename = { "nodex-agent.exe" };

        let agent_path = output_path.join(agent_filename);

        let response = reqwest::get(binary_url).await?;
        let content = response.bytes().await?;
        if PathBuf::from(&agent_path).exists() {
            fs::remove_file(&agent_path)?;
        }
        self.extract_zip(content, &output_path)?;

        #[cfg(unix)]
        {
            let home_dir = dirs::home_dir().unwrap();
            let path = home_dir.join(".nodex").join("runtime_info.json");
            let mut runtime_info = RuntimeInfo::new(path);
            runtime_info.read()?;
            let resource_manager = ResourceManager::new();
            resource_manager.backup()?;
            self.run_controller(&agent_path, &mut runtime_info)?;
            runtime_info.update_state(State::Updating)?;
        }

        #[cfg(windows)]
        self.run_agent(&agent_path)?;

        Ok(())
    }

    fn extract_zip(&self, archive_data: Bytes, output_path: &Path) -> anyhow::Result<()> {
        let cursor = Cursor::new(archive_data);
        let mut archive = ZipArchive::new(cursor)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = output_path.join(file.mangled_name());

            if file.is_file() {
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut output_file = fs::File::create(&file_path)?;
                std::io::copy(&mut file, &mut output_file)?;
            } else if file.is_dir() {
                std::fs::create_dir_all(&file_path)?;
            }
        }

        Ok(())
    }

    #[cfg(unix)]
    fn run_controller(
        &self,
        agent_path: &Path,
        runtime_info: &mut RuntimeInfo,
    ) -> anyhow::Result<()> {
        let mut process_ids_to_remove = vec![];
        for process_info in &runtime_info.process_infos {
            if process_info.feat_type == FeatType::Controller {
                signal::kill(
                    Pid::from_raw(process_info.process_id as i32),
                    Signal::SIGTERM,
                )
                .map_err(|e| {
                    anyhow::anyhow!("Failed to kill process {}: {}", process_info.process_id, e)
                })?;
                process_ids_to_remove.push(process_info.process_id);
            }
        }

        for process_id in process_ids_to_remove {
            runtime_info.remove_process_info(process_id);
        }

        if is_manage_by_systemd() && is_manage_socket_activation() {
            return Ok(());
        }

        Command::new("chmod").arg("+x").arg(agent_path).status()?;
        let daemonize = Daemonize::new();
        daemonize
            .start()
            .map_err(|e| anyhow::anyhow!("Failed to update nodexe_pro process: {}", e))?;
        std::process::Command::new(agent_path)
            .arg("controller")
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;
        Ok(())
    }

    #[cfg(windows)]
    fn run_agent(&self, agent_path: &Path) -> anyhow::Result<()> {
        let agent_path_str = agent_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to convert agent_path to string"))?;

        let status = Command::new("cmd")
            .args(&["/C", "start", agent_path_str])
            .status()?;

        if !status.success() {
            eprintln!("Command execution failed with status: {}", status);
        } else {
            println!("Started child process");
        }

        Ok(())
    }
}
