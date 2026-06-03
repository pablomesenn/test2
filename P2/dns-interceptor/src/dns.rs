use std::net::Ipv4Addr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DnsError {
    #[error("mensaje muy corto: {0} bytes")]
    TooShort(usize),
    #[error("nombre de dominio inválido")]
    InvalidName,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DnsQuery {
    pub id: u16,
    pub name: String,
    pub raw: Vec<u8>,
}

// Parsea el nombre de dominio desde el wire format de DNS
// Wire format: secuencia de (longitud, bytes) terminada en 0x00
pub fn parse_query(buf: &[u8]) -> Result<DnsQuery, DnsError> {
    if buf.len() < 12 {
        return Err(DnsError::TooShort(buf.len()));
    }

    let id = u16::from_be_bytes([buf[0], buf[1]]);
    let name = parse_name(buf, 12)?;

    Ok(DnsQuery {
        id,
        name,
        raw: buf.to_vec(),
    })
}

fn parse_name(buf: &[u8], mut offset: usize) -> Result<String, DnsError> {
    let mut labels = Vec::new();

    loop {
        if offset >= buf.len() {
            return Err(DnsError::InvalidName);
        }
        let len = buf[offset] as usize;
        if len == 0 {
            break;
        }
        // Detectar compresión DNS (los dos bits altos = 11)
        if len & 0xC0 == 0xC0 {
            if offset + 1 >= buf.len() {
                return Err(DnsError::InvalidName);
            }
            let ptr = (((len & 0x3F) as usize) << 8) | buf[offset + 1] as usize;
            return parse_name(buf, ptr);
        }
        offset += 1;
        if offset + len > buf.len() {
            return Err(DnsError::InvalidName);
        }
        let label = std::str::from_utf8(&buf[offset..offset + len])
            .map_err(|_| DnsError::InvalidName)?;
        labels.push(label.to_lowercase());
        offset += len;
    }

    Ok(labels.join("."))
}

/// Construye una respuesta DNS tipo A apuntando a una IP específica.
/// Reutiliza el mismo ID y la pregunta original del cliente.
pub fn build_a_response(query: &DnsQuery, ip: Ipv4Addr) -> Vec<u8> {
    let mut resp = Vec::with_capacity(512);

    // Header
    resp.extend_from_slice(&query.id.to_be_bytes()); // ID
    resp.extend_from_slice(&[0x84, 0x00]);            // flags: QR=1, AA=1, RD=1
    resp.extend_from_slice(&[0x00, 0x01]);            // QDCOUNT = 1
    resp.extend_from_slice(&[0x00, 0x01]);            // ANCOUNT = 1
    resp.extend_from_slice(&[0x00, 0x00]);            // NSCOUNT = 0
    resp.extend_from_slice(&[0x00, 0x00]);            // ARCOUNT = 0 (sin OPT record)

    // Question section: reconstruida limpia desde el nombre parseado
    for label in query.name.split('.') {
        resp.push(label.len() as u8);
        resp.extend_from_slice(label.as_bytes());
    }
    resp.push(0x00);                    // fin del nombre
    resp.extend_from_slice(&[0x00, 0x01]); // QTYPE  = A
    resp.extend_from_slice(&[0x00, 0x01]); // QCLASS = IN

    // Answer section
    resp.extend_from_slice(&[0xC0, 0x0C]); // puntero al nombre (offset 12)
    resp.extend_from_slice(&[0x00, 0x01]); // TYPE  = A
    resp.extend_from_slice(&[0x00, 0x01]); // CLASS = IN
    resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x1E]); // TTL = 30s
    resp.extend_from_slice(&[0x00, 0x04]); // RDLENGTH = 4
    resp.extend_from_slice(&ip.octets());  // RDATA

    resp
}

/// Verifica si el nombre consultado pertenece a alguno de los dominios gestionados
pub fn is_managed(name: &str, managed_domains: &[String]) -> bool {
    managed_domains.iter().any(|domain| {
        name == domain || name.ends_with(&format!(".{}", domain))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_query(name: &str) -> Vec<u8> {
        let mut buf = vec![
            0xAB, 0xCD, // ID
            0x01, 0x00, // flags: query estándar
            0x00, 0x01, // QDCOUNT = 1
            0x00, 0x00, // ANCOUNT
            0x00, 0x00, // NSCOUNT
            0x00, 0x00, // ARCOUNT
        ];
        for label in name.split('.') {
            buf.push(label.len() as u8);
            buf.extend_from_slice(label.as_bytes());
        }
        buf.push(0x00); // fin del nombre
        buf.extend_from_slice(&[0x00, 0x01]); // QTYPE A
        buf.extend_from_slice(&[0x00, 0x01]); // QCLASS IN
        buf
    }

    #[test]
    fn test_parse_query_simple() {
        let buf = make_query("example.com");
        let q = parse_query(&buf).unwrap();
        assert_eq!(q.id, 0xABCD);
        assert_eq!(q.name, "example.com");
    }

    #[test]
    fn test_parse_query_subdomain() {
        let buf = make_query("api.example.com");
        let q = parse_query(&buf).unwrap();
        assert_eq!(q.name, "api.example.com");
    }

    #[test]
    fn test_parse_query_too_short() {
        let buf = vec![0x00; 5];
        assert!(matches!(parse_query(&buf), Err(DnsError::TooShort(5))));
    }

    #[test]
    fn test_parse_query_normalizes_case() {
        let buf = make_query("Example.COM");
        let q = parse_query(&buf).unwrap();
        assert_eq!(q.name, "example.com");
    }

    #[test]
    fn test_build_a_response_length() {
        let buf = make_query("example.com");
        let q = parse_query(&buf).unwrap();
        let resp = build_a_response(&q, Ipv4Addr::new(10, 0, 0, 1));
        // Header(12) + question + answer(16)
        assert!(resp.len() > 12);
    }

    #[test]
    fn test_build_a_response_contains_ip() {
        let buf = make_query("example.com");
        let q = parse_query(&buf).unwrap();
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        let resp = build_a_response(&q, ip);
        let octets = ip.octets();
        assert!(resp.windows(4).any(|w| w == octets));
    }

    #[test]
    fn test_is_managed_exact() {
        let domains = vec!["example.com".to_string()];
        assert!(is_managed("example.com", &domains));
    }

    #[test]
    fn test_is_managed_subdomain() {
        let domains = vec!["example.com".to_string()];
        assert!(is_managed("api.example.com", &domains));
        assert!(is_managed("sub.api.example.com", &domains));
    }

    #[test]
    fn test_is_managed_not_managed() {
        let domains = vec!["example.com".to_string()];
        assert!(!is_managed("notexample.com", &domains));
        assert!(!is_managed("other.org", &domains));
    }
}
