use crate::firestore::AuthMode;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("API key inválida")]
    InvalidApiKey,
    #[error("credenciales inválidas")]
    InvalidCredentials,
    #[error("sesión expirada o inválida")]
    InvalidSession,
    #[error("no autorizado")]
    Unauthorized,
    #[error("modo de autenticación incorrecto: se requiere {required}")]
    WrongMode { required: String },
}

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub expires_at: u64,
}

pub struct AuthManager {
    // api_key → user_id
    api_keys: HashMap<String, String>,
    // user_id → hashed_password
    users: HashMap<String, String>,
    // session_token → Session
    sessions: HashMap<String, Session>,
    session_ttl_secs: u64,
}

impl AuthManager {
    pub fn new(session_ttl_secs: u64) -> Self {
        Self {
            api_keys: HashMap::new(),
            users: HashMap::new(),
            sessions: HashMap::new(),
            session_ttl_secs,
        }
    }

    fn now_ts() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn register_api_key(&mut self, api_key: &str, user_id: &str) {
        self.api_keys
            .insert(api_key.to_string(), user_id.to_string());
    }

    pub fn register_user(&mut self, user_id: &str, password: &str) {
        self.users
            .insert(user_id.to_string(), Self::hash_password(password));
    }

    /// Valida un API key — devuelve el user_id si es válido
    pub fn validate_api_key(&self, api_key: &str) -> Result<String, AuthError> {
        self.api_keys
            .get(api_key)
            .cloned()
            .ok_or(AuthError::InvalidApiKey)
    }

    /// Login con usuario/contraseña — devuelve un token de sesión
    pub fn login(&mut self, user_id: &str, password: &str) -> Result<String, AuthError> {
        let hashed = self
            .users
            .get(user_id)
            .ok_or(AuthError::InvalidCredentials)?;

        if *hashed != Self::hash_password(password) {
            return Err(AuthError::InvalidCredentials);
        }

        let token = Uuid::new_v4().to_string();
        let expires_at = Self::now_ts() + self.session_ttl_secs;

        self.sessions.insert(
            token.clone(),
            Session {
                token: token.clone(),
                user_id: user_id.to_string(),
                expires_at,
            },
        );

        Ok(token)
    }

    /// Valida un token de sesión — devuelve el user_id si es válido y no expiró
    pub fn validate_session(&self, token: &str) -> Result<String, AuthError> {
        let session = self.sessions.get(token).ok_or(AuthError::InvalidSession)?;

        if Self::now_ts() > session.expires_at {
            return Err(AuthError::InvalidSession);
        }

        Ok(session.user_id.clone())
    }

    /// Invalida una sesión (logout)
    pub fn logout(&mut self, token: &str) {
        self.sessions.remove(token);
    }

    /// Limpia sesiones expiradas
    pub fn cleanup_sessions(&mut self) {
        let now = Self::now_ts();
        self.sessions.retain(|_, s| s.expires_at > now);
    }

    /// Valida credenciales según el modo exigido por la ruta.
    /// Esto refuerza el contrato del spec: una ruta marcada como API_KEY
    /// no debería poder accederse con un x-session-token.
    pub fn validate_for_mode(
        &self,
        mode: &AuthMode,
        api_key: Option<&str>,
        session_token: Option<&str>,
    ) -> Result<Option<String>, AuthError> {
        match mode {
            AuthMode::None => Ok(None),
            AuthMode::ApiKey => {
                let key = api_key.ok_or(AuthError::WrongMode {
                    required: "API_KEY".into(),
                })?;
                self.validate_api_key(key).map(Some)
            }
            AuthMode::UserPassword => {
                let token = session_token.ok_or(AuthError::WrongMode {
                    required: "USER_PASSWORD".into(),
                })?;
                self.validate_session(token).map(Some)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auth() -> AuthManager {
        let mut auth = AuthManager::new(3600);
        auth.register_user("user1", "password123");
        auth.register_api_key("key-abc-123", "user1");
        auth
    }

    #[test]
    fn test_valid_api_key() {
        let auth = make_auth();
        assert_eq!(auth.validate_api_key("key-abc-123").unwrap(), "user1");
    }

    #[test]
    fn test_invalid_api_key() {
        let auth = make_auth();
        assert!(matches!(
            auth.validate_api_key("wrong"),
            Err(AuthError::InvalidApiKey)
        ));
    }

    #[test]
    fn test_login_success() {
        let mut auth = make_auth();
        let token = auth.login("user1", "password123").unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_login_wrong_password() {
        let mut auth = make_auth();
        assert!(matches!(
            auth.login("user1", "wrong"),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn test_login_unknown_user() {
        let mut auth = make_auth();
        assert!(matches!(
            auth.login("nobody", "x"),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn test_session_valid_after_login() {
        let mut auth = make_auth();
        let token = auth.login("user1", "password123").unwrap();
        assert_eq!(auth.validate_session(&token).unwrap(), "user1");
    }

    #[test]
    fn test_invalid_session_token() {
        let auth = make_auth();
        assert!(matches!(
            auth.validate_session("fake"),
            Err(AuthError::InvalidSession)
        ));
    }

    #[test]
    fn test_logout_invalidates_session() {
        let mut auth = make_auth();
        let token = auth.login("user1", "password123").unwrap();
        auth.logout(&token);
        assert!(matches!(
            auth.validate_session(&token),
            Err(AuthError::InvalidSession)
        ));
    }

    #[test]
    fn test_hash_password_deterministic() {
        assert_eq!(
            AuthManager::hash_password("s"),
            AuthManager::hash_password("s")
        );
    }

    #[test]
    fn test_hash_password_different() {
        assert_ne!(
            AuthManager::hash_password("a"),
            AuthManager::hash_password("b")
        );
    }

    #[test]
    fn test_cleanup_removes_expired() {
        let mut auth = AuthManager::new(0);
        auth.register_user("u", "p");
        let token = auth.login("u", "p").unwrap();
        auth.cleanup_sessions();
        assert!(matches!(
            auth.validate_session(&token),
            Err(AuthError::InvalidSession)
        ));
    }

    #[test]
    fn test_validate_for_mode_none_allows_anonymous() {
        let auth = make_auth();
        let r = auth.validate_for_mode(&AuthMode::None, None, None);
        assert!(matches!(r, Ok(None)));
    }

    #[test]
    fn test_validate_for_mode_api_key_with_valid_key() {
        let auth = make_auth();
        let r = auth.validate_for_mode(&AuthMode::ApiKey, Some("key-abc-123"), None);
        assert_eq!(r.unwrap(), Some("user1".to_string()));
    }

    #[test]
    fn test_validate_for_mode_api_key_missing_rejects() {
        let auth = make_auth();
        let r = auth.validate_for_mode(&AuthMode::ApiKey, None, Some("some-session"));
        assert!(matches!(r, Err(AuthError::WrongMode { .. })));
    }

    #[test]
    fn test_validate_for_mode_user_password_with_valid_session() {
        let mut auth = make_auth();
        let token = auth.login("user1", "password123").unwrap();
        let r = auth.validate_for_mode(&AuthMode::UserPassword, None, Some(&token));
        assert_eq!(r.unwrap(), Some("user1".to_string()));
    }

    #[test]
    fn test_validate_for_mode_user_password_missing_rejects() {
        let auth = make_auth();
        let r = auth.validate_for_mode(&AuthMode::UserPassword, Some("k"), None);
        assert!(matches!(r, Err(AuthError::WrongMode { .. })));
    }
}
