use bitcoin::secp256k1::PublicKey;
use lightning::routing::gossip::NodeId;
use std::net::AddrParseError;
use std::net::SocketAddr;
use std::str::FromStr;
use thiserror::Error;

/// LightningNodeAddr represents a lightning node address in the form "0288037d3f0bdcfb240402b43b80cdc32e41528b3e2ebe05884aff507d71fca71a@161.97.184.185:9735"

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct LightningNodeAddr {
    pub node_id: NodeId,
    pub endpoint: SocketAddr,
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum LightningNodeAddrError {
    #[error("Parse error")]
    ParseError,
    #[error("Key error {0}")]
    KeyError(#[from] bitcoin::secp256k1::Error),
    #[error("Address error {0}")]
    AddressError(#[from] AddrParseError),
}

impl FromStr for LightningNodeAddr {
    type Err = LightningNodeAddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let chunks: Vec<&str> = s.split("@").collect();
        if chunks.len() != 2 {
            return Err(LightningNodeAddrError::ParseError);
        }

        Ok(Self {
            node_id: NodeId::from_pubkey(&PublicKey::from_str(chunks.get(0).unwrap())?),
            endpoint: chunks.get(1).unwrap().parse()?,
        })
    }
}

impl std::fmt::Display for LightningNodeAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        return write!(f, "{}@{}", self.node_id, self.endpoint);
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct LightningNodeAddrVec(pub Vec<LightningNodeAddr>);

impl std::fmt::Display for LightningNodeAddrVec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut comma_separated = String::new();

        for one in &self.0[0..self.0.len() - 1] {
            comma_separated.push_str(&one.to_string());
            comma_separated.push_str(",");
        }

        comma_separated.push_str(&self.0[self.0.len() - 1].to_string());
        return write!(f, "{}", comma_separated);
    }
}

impl std::ops::Deref for LightningNodeAddrVec {
    type Target = Vec<LightningNodeAddr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for LightningNodeAddrVec {
    type Err = LightningNodeAddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result: Result<Vec<LightningNodeAddr>, LightningNodeAddrError> = s
            .split(',')
            .map(str::trim)
            .map(|part| LightningNodeAddr::from_str(part))
            .collect();

        if let Ok(ret) = result {
            return Ok(LightningNodeAddrVec(ret));
        } else if let Err(err) = result {
            return Err(err);
        }

        return Err(LightningNodeAddrError::ParseError);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::Error::InvalidPublicKey;
    use std::str::FromStr;

    use std::any::type_name;

    fn type_of<T>(_: T) -> &'static str {
        type_name::<T>()
    }

    #[test]
    fn test_addr_from_str() {
        for s in ["02864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@[::1]:1337", "0288037d3f0bdcfb240402b43b80cdc32e41528b3e2ebe05884aff507d71fca71a@161.97.184.185:9735",
            "03864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@1.2.3.4:1337", "0327f763c849bfd218910e41eef74f5a737989358ab3565f185e1a61bb7df445b8@1.2.3.4:9735"].iter() {
                LightningNodeAddr::from_str(s).expect("Should be valid");
            }

        assert_eq!(
            LightningNodeAddrError::ParseError,
            LightningNodeAddr::from_str("foo").unwrap_err()
        );

        if let LightningNodeAddrError::KeyError(t) = LightningNodeAddr::from_str(
            "04864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@1.2.3.4:9735",
        )
        .unwrap_err()
        {
            assert_eq!(InvalidPublicKey, t)
        } else {
            panic!("Expected KeyError");
        }

        if let LightningNodeAddrError::AddressError(t) = LightningNodeAddr::from_str(
            "03864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@1.2.3.4",
        )
        .unwrap_err()
        {
            assert_eq!("core::net::parser::AddrParseError", type_of(t));
        } else {
            panic!("Expected AddressError");
        }
    }
}
