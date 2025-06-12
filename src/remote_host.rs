use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteHost {
    pub name: String,
    pub hostname: String,
    pub username: String,
    pub auth_type: AuthType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    Password,
    Key { path: Option<PathBuf> },
}

impl RemoteHost {
    pub fn new(name: String, hostname: String, username: String, auth_type: AuthType) -> Self {
        Self {
            name,
            hostname,
            username,
            auth_type,
        }
    }

    pub fn connection_string(&self) -> String {
        format!("{}@{}", self.username, self.hostname)
    }

    pub fn display_name(&self) -> String {
        format!("{} ({})", self.name, self.connection_string())
    }

    pub fn is_password_auth(&self) -> bool {
        matches!(self.auth_type, AuthType::Password)
    }

    pub fn is_key_auth(&self) -> bool {
        matches!(self.auth_type, AuthType::Key { .. })
    }

    pub fn key_path(&self) -> Option<&PathBuf> {
        match &self.auth_type {
            AuthType::Key { path } => path.as_ref(),
            _ => None,
        }
    }
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthType::Password => write!(f, "Password"),
            AuthType::Key { path } => {
                if let Some(p) = path {
                    write!(f, "SSH Key ({})", p.display())
                } else {
                    write!(f, "SSH Key (default)")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_host_creation() {
        let host = RemoteHost::new(
            "test-server".to_string(),
            "example.com".to_string(),
            "user".to_string(),
            AuthType::Password,
        );

        assert_eq!(host.name, "test-server");
        assert_eq!(host.hostname, "example.com");
        assert_eq!(host.username, "user");
        assert!(host.is_password_auth());
        assert!(!host.is_key_auth());
    }

    #[test]
    fn test_connection_string() {
        let host = RemoteHost::new(
            "test-server".to_string(),
            "example.com".to_string(),
            "user".to_string(),
            AuthType::Password,
        );

        assert_eq!(host.connection_string(), "user@example.com");
    }

    #[test]
    fn test_key_auth() {
        let key_path = PathBuf::from("/home/user/.ssh/id_rsa");
        let host = RemoteHost::new(
            "test-server".to_string(),
            "example.com".to_string(),
            "user".to_string(),
            AuthType::Key {
                path: Some(key_path.clone()),
            },
        );

        assert!(host.is_key_auth());
        assert!(!host.is_password_auth());
        assert_eq!(host.key_path(), Some(&key_path));
    }

    #[test]
    fn test_serialization() {
        let host = RemoteHost::new(
            "test-server".to_string(),
            "example.com".to_string(),
            "user".to_string(),
            AuthType::Password,
        );

        let json = serde_json::to_string(&host).unwrap();
        let deserialized: RemoteHost = serde_json::from_str(&json).unwrap();

        assert_eq!(host.name, deserialized.name);
        assert_eq!(host.hostname, deserialized.hostname);
        assert_eq!(host.username, deserialized.username);
    }
}
