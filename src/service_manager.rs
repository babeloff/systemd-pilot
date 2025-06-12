use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub status: ServiceStatus,
    pub description: Option<String>,
    pub enabled: bool,
    pub active: bool,
    pub load_state: String,
    pub sub_state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Active,
    Inactive,
    Failed,
    Unknown,
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStatus::Active => write!(f, "Active"),
            ServiceStatus::Inactive => write!(f, "Inactive"),
            ServiceStatus::Failed => write!(f, "Failed"),
            ServiceStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<&str> for ServiceStatus {
    fn from(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "active" => ServiceStatus::Active,
            "inactive" => ServiceStatus::Inactive,
            "failed" => ServiceStatus::Failed,
            _ => ServiceStatus::Unknown,
        }
    }
}

pub struct ServiceManager {
    runtime: Arc<Runtime>,
}

impl ServiceManager {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }

    pub async fn list_local_services(&self, show_inactive: bool) -> Result<Vec<ServiceInfo>> {
        let mut cmd = TokioCommand::new("systemctl");
        cmd.args(&["list-units", "--type=service", "--no-pager"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if show_inactive {
            cmd.arg("--all");
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to list services: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_service_list(&stdout)
    }

    pub async fn get_service_status(&self, service_name: &str) -> Result<ServiceInfo> {
        let cmd = TokioCommand::new("systemctl")
            .args(&["show", service_name, "--no-pager"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !cmd.status.success() {
            let stderr = String::from_utf8_lossy(&cmd.stderr);
            return Err(anyhow!("Failed to get service status: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&cmd.stdout);
        self.parse_service_status(service_name, &stdout)
    }

    pub async fn start_service(&self, service_name: &str) -> Result<()> {
        self.run_systemctl_command(&["start", service_name]).await
    }

    pub async fn stop_service(&self, service_name: &str) -> Result<()> {
        self.run_systemctl_command(&["stop", service_name]).await
    }

    pub async fn restart_service(&self, service_name: &str) -> Result<()> {
        self.run_systemctl_command(&["restart", service_name]).await
    }

    pub async fn enable_service(&self, service_name: &str) -> Result<()> {
        self.run_systemctl_command(&["enable", service_name]).await
    }

    pub async fn disable_service(&self, service_name: &str) -> Result<()> {
        self.run_systemctl_command(&["disable", service_name]).await
    }

    pub async fn reload_service(&self, service_name: &str) -> Result<()> {
        self.run_systemctl_command(&["reload", service_name]).await
    }

    pub async fn get_service_logs(&self, service_name: &str, lines: Option<u32>) -> Result<String> {
        let mut cmd = TokioCommand::new("journalctl");
        cmd.args(&["-u", service_name, "--no-pager"]);

        if let Some(n) = lines {
            cmd.args(&["-n", &n.to_string()]);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get service logs: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn daemon_reload(&self) -> Result<()> {
        self.run_systemctl_command(&["daemon-reload"]).await
    }

    pub async fn create_service_file(&self, service_name: &str, content: &str) -> Result<()> {
        let service_path = format!("/etc/systemd/system/{}.service", service_name);

        // Write service file (requires sudo)
        let mut cmd = TokioCommand::new("sudo");
        cmd.args(&["tee", &service_path])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(content.as_bytes()).await?;
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create service file: {}", stderr));
        }

        // Reload systemd after creating new service
        self.daemon_reload().await?;

        Ok(())
    }

    async fn run_systemctl_command(&self, args: &[&str]) -> Result<()> {
        let cmd = TokioCommand::new("systemctl")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !cmd.status.success() {
            let stderr = String::from_utf8_lossy(&cmd.stderr);
            return Err(anyhow!("systemctl command failed: {}", stderr));
        }

        Ok(())
    }

    fn parse_service_list(&self, output: &str) -> Result<Vec<ServiceInfo>> {
        let mut services = Vec::new();
        let lines: Vec<&str> = output.lines().collect();

        // Skip header lines and find the start of service listings
        let mut start_idx = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("UNIT") {
                start_idx = i + 1;
                break;
            }
        }

        for line in lines.iter().skip(start_idx) {
            if line.trim().is_empty() || line.starts_with("LOAD") {
                break;
            }

            if let Some(service) = self.parse_service_line(line) {
                services.push(service);
            }
        }

        Ok(services)
    }

    fn parse_service_line(&self, line: &str) -> Option<ServiceInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let name = parts[0].trim_end_matches(".service").to_string();
        let load_state = parts[1].to_string();
        let active_state = parts[2].to_string();
        let sub_state = parts[3].to_string();

        let description = if parts.len() > 4 {
            Some(parts[4..].join(" "))
        } else {
            None
        };

        let status = ServiceStatus::from(active_state.as_str());
        let active = active_state == "active";

        Some(ServiceInfo {
            name,
            status,
            description,
            enabled: false, // This would need a separate query
            active,
            load_state,
            sub_state,
        })
    }

    fn parse_service_status(&self, service_name: &str, output: &str) -> Result<ServiceInfo> {
        let mut properties = HashMap::new();

        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                properties.insert(key.trim(), value.trim());
            }
        }

        let active_state = properties.get("ActiveState").unwrap_or(&"unknown");
        let sub_state = properties.get("SubState").unwrap_or(&"unknown");
        let load_state = properties.get("LoadState").unwrap_or(&"unknown");
        let unit_file_state = properties.get("UnitFileState").unwrap_or(&"unknown");
        let description = properties.get("Description").map(|s| s.to_string());

        let status = ServiceStatus::from(*active_state);
        let active = *active_state == "active";
        let enabled = *unit_file_state == "enabled";

        Ok(ServiceInfo {
            name: service_name.to_string(),
            status,
            description,
            enabled,
            active,
            load_state: load_state.to_string(),
            sub_state: sub_state.to_string(),
        })
    }
}

// Remote service management
pub struct RemoteServiceManager {
    session: ssh2::Session,
}

impl RemoteServiceManager {
    pub fn new(session: ssh2::Session) -> Self {
        Self { session }
    }

    pub async fn list_services(&self, show_inactive: bool) -> Result<Vec<ServiceInfo>> {
        let mut command = "systemctl list-units --type=service --no-pager".to_string();
        if show_inactive {
            command.push_str(" --all");
        }

        let output = self.execute_command(&command).await?;
        self.parse_service_list(&output)
    }

    pub async fn get_service_status(&self, service_name: &str) -> Result<ServiceInfo> {
        let command = format!("systemctl show {} --no-pager", service_name);
        let output = self.execute_command(&command).await?;
        self.parse_service_status(service_name, &output)
    }

    pub async fn start_service(&self, service_name: &str) -> Result<()> {
        let command = format!("sudo systemctl start {}", service_name);
        self.execute_command(&command).await?;
        Ok(())
    }

    pub async fn stop_service(&self, service_name: &str) -> Result<()> {
        let command = format!("sudo systemctl stop {}", service_name);
        self.execute_command(&command).await?;
        Ok(())
    }

    pub async fn restart_service(&self, service_name: &str) -> Result<()> {
        let command = format!("sudo systemctl restart {}", service_name);
        self.execute_command(&command).await?;
        Ok(())
    }

    pub async fn enable_service(&self, service_name: &str) -> Result<()> {
        let command = format!("sudo systemctl enable {}", service_name);
        self.execute_command(&command).await?;
        Ok(())
    }

    pub async fn disable_service(&self, service_name: &str) -> Result<()> {
        let command = format!("sudo systemctl disable {}", service_name);
        self.execute_command(&command).await?;
        Ok(())
    }

    pub async fn get_service_logs(&self, service_name: &str, lines: Option<u32>) -> Result<String> {
        let mut command = format!("journalctl -u {} --no-pager", service_name);
        if let Some(n) = lines {
            command.push_str(&format!(" -n {}", n));
        }

        self.execute_command(&command).await
    }

    async fn execute_command(&self, command: &str) -> Result<String> {
        // This is a simplified version - in practice you'd need proper async SSH handling
        // For now, we'll use a blocking approach wrapped in spawn_blocking
        let command = command.to_string();

        tokio::task::spawn_blocking(move || {
            // SSH command execution would go here
            // This is a placeholder implementation
            Ok("".to_string())
        })
        .await?
    }

    fn parse_service_list(&self, output: &str) -> Result<Vec<ServiceInfo>> {
        // Same parsing logic as local service manager
        let mut services = Vec::new();
        let lines: Vec<&str> = output.lines().collect();

        let mut start_idx = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("UNIT") {
                start_idx = i + 1;
                break;
            }
        }

        for line in lines.iter().skip(start_idx) {
            if line.trim().is_empty() || line.starts_with("LOAD") {
                break;
            }

            if let Some(service) = self.parse_service_line(line) {
                services.push(service);
            }
        }

        Ok(services)
    }

    fn parse_service_line(&self, line: &str) -> Option<ServiceInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let name = parts[0].trim_end_matches(".service").to_string();
        let load_state = parts[1].to_string();
        let active_state = parts[2].to_string();
        let sub_state = parts[3].to_string();

        let description = if parts.len() > 4 {
            Some(parts[4..].join(" "))
        } else {
            None
        };

        let status = ServiceStatus::from(active_state.as_str());
        let active = active_state == "active";

        Some(ServiceInfo {
            name,
            status,
            description,
            enabled: false,
            active,
            load_state,
            sub_state,
        })
    }

    fn parse_service_status(&self, service_name: &str, output: &str) -> Result<ServiceInfo> {
        let mut properties = HashMap::new();

        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                properties.insert(key.trim(), value.trim());
            }
        }

        let active_state = properties.get("ActiveState").unwrap_or(&"unknown");
        let sub_state = properties.get("SubState").unwrap_or(&"unknown");
        let load_state = properties.get("LoadState").unwrap_or(&"unknown");
        let unit_file_state = properties.get("UnitFileState").unwrap_or(&"unknown");
        let description = properties.get("Description").map(|s| s.to_string());

        let status = ServiceStatus::from(*active_state);
        let active = *active_state == "active";
        let enabled = *unit_file_state == "enabled";

        Ok(ServiceInfo {
            name: service_name.to_string(),
            status,
            description,
            enabled,
            active,
            load_state: load_state.to_string(),
            sub_state: sub_state.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_status_parsing() {
        assert_eq!(ServiceStatus::from("active"), ServiceStatus::Active);
        assert_eq!(ServiceStatus::from("inactive"), ServiceStatus::Inactive);
        assert_eq!(ServiceStatus::from("failed"), ServiceStatus::Failed);
        assert_eq!(ServiceStatus::from("unknown"), ServiceStatus::Unknown);
    }

    #[test]
    fn test_service_status_display() {
        assert_eq!(format!("{}", ServiceStatus::Active), "Active");
        assert_eq!(format!("{}", ServiceStatus::Inactive), "Inactive");
        assert_eq!(format!("{}", ServiceStatus::Failed), "Failed");
        assert_eq!(format!("{}", ServiceStatus::Unknown), "Unknown");
    }
}
