use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientIdentity {
    ApiKey { raw: String, fingerprint: String },
    IpAddress(IpAddr),
}

#[derive(Debug)]
pub enum AuthError {
    InvalidScheme,
}

impl ClientIdentity {
    /// Определяет клиента по заголовкам и IP
    pub fn from_request(headers: &HeaderMap, ip: IpAddr) -> Result<Self, AuthError> {
        if let Some(key) = api_key_from_headers(headers)? {
            return Ok(ClientIdentity::ApiKey {
                fingerprint: fingerprint(&key),
                raw: key,
            });
        }

        Ok(ClientIdentity::IpAddress(ip))
    }

    pub fn api_key_raw(&self) -> Option<&str> {
        match self {
            ClientIdentity::ApiKey { raw, .. } => Some(raw.as_str()),
            ClientIdentity::IpAddress(_) => None,
        }
    }
}

impl std::fmt::Display for ClientIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientIdentity::ApiKey { fingerprint, .. } => write!(f, "apikey:{fingerprint}"),
            ClientIdentity::IpAddress(ip) => write!(f, "ip:{ip}"),
        }
    }
}

fn api_key_from_headers(headers: &HeaderMap) -> Result<Option<String>, AuthError> {
    if let Some(auth_header) = headers.get("authorization") {
        let auth_str = auth_header.to_str().map_err(|_| AuthError::InvalidScheme)?;
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            return Ok(Some(token.to_string()));
        }
        return Err(AuthError::InvalidScheme);
    }

    if let Some(api_key) = headers.get("x-api-key") {
        let key_str = api_key.to_str().map_err(|_| AuthError::InvalidScheme)?;
        return Ok(Some(key_str.to_string()));
    }

    Ok(None)
}

pub fn fingerprint(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        out.push_str(&format!("{:02x}", byte));
    }
    format!("fp_{out}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::net::IpAddr;

    #[test]
    fn test_api_key_from_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer abc123".parse().unwrap());
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        let identity = ClientIdentity::from_request(&headers, ip).unwrap();
        assert_eq!(
            identity,
            ClientIdentity::ApiKey {
                raw: "abc123".to_string(),
                fingerprint: fingerprint("abc123")
            }
        );
    }

    #[test]
    fn test_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "mykey456".parse().unwrap());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        let identity = ClientIdentity::from_request(&headers, ip).unwrap();
        assert_eq!(
            identity,
            ClientIdentity::ApiKey {
                raw: "mykey456".to_string(),
                fingerprint: fingerprint("mykey456")
            }
        );
    }

    #[test]
    fn test_fallback_to_ip() {
        let headers = HeaderMap::new();
        let ip: IpAddr = "10.0.0.5".parse().unwrap();

        let identity = ClientIdentity::from_request(&headers, ip).unwrap();
        assert_eq!(identity, ClientIdentity::IpAddress(ip));
    }

    #[test]
    fn test_invalid_auth_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Basic abc123".parse().unwrap());
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        let identity = ClientIdentity::from_request(&headers, ip);
        assert!(matches!(identity, Err(AuthError::InvalidScheme)));
    }
}
