use axum::http::HeaderMap;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientIdentity {
    ApiKey(String),
    IpAddress(IpAddr),
}

impl ClientIdentity {
    /// Определяет клиента по заголовкам и IP
    pub fn from_request(headers: &HeaderMap, ip: IpAddr) -> Self {
        // Проверяем Authorization header
        if let Some(auth_header) = headers.get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                // Поддержка Bearer токенов
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    return ClientIdentity::ApiKey(token.to_string());
                }
                // Поддержка простых API ключей
                return ClientIdentity::ApiKey(auth_str.to_string());
            }
        }

        // Проверяем X-API-Key header
        if let Some(api_key) = headers.get("x-api-key") {
            if let Ok(key_str) = api_key.to_str() {
                return ClientIdentity::ApiKey(key_str.to_string());
            }
        }

        // Fallback на IP адрес
        ClientIdentity::IpAddress(ip)
    }
}

impl std::fmt::Display for ClientIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientIdentity::ApiKey(key) => write!(f, "apikey:{key}"),
            ClientIdentity::IpAddress(ip) => write!(f, "ip:{ip}"),
        }
    }
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

        let identity = ClientIdentity::from_request(&headers, ip);
        assert_eq!(identity, ClientIdentity::ApiKey("abc123".to_string()));
    }

    #[test]
    fn test_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "mykey456".parse().unwrap());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        let identity = ClientIdentity::from_request(&headers, ip);
        assert_eq!(identity, ClientIdentity::ApiKey("mykey456".to_string()));
    }

    #[test]
    fn test_fallback_to_ip() {
        let headers = HeaderMap::new();
        let ip: IpAddr = "10.0.0.5".parse().unwrap();

        let identity = ClientIdentity::from_request(&headers, ip);
        assert_eq!(identity, ClientIdentity::IpAddress(ip));
    }
}
