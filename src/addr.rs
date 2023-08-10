mod addr {
    use lightning::routing::gossip::NodeId;
    use std::str::FromStr;
    use bitcoin::secp256k1::PublicKey;
    use std::net::AddrParseError;
    use std::net::SocketAddr;
    use thiserror::Error;
    

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

            Ok(Self{ node_id: NodeId::from_pubkey(&PublicKey::from_str(chunks.get(0).unwrap())?), endpoint: chunks.get(1).unwrap().parse()? })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::addr::*;
    use std::str::FromStr;
    use bitcoin::secp256k1::Error::InvalidPublicKey;
    
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
        
        assert_eq!(LightningNodeAddrError::ParseError, LightningNodeAddr::from_str("foo").unwrap_err());

        if let LightningNodeAddrError::KeyError(t) = LightningNodeAddr::from_str("04864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@1.2.3.4:9735").unwrap_err() {
            assert_eq!(InvalidPublicKey, t)
        } else {
            panic!("Expected KeyError");
        }

        if let LightningNodeAddrError::AddressError(t) = LightningNodeAddr::from_str("03864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@1.2.3.4").unwrap_err() {
            assert_eq!("core::net::parser::AddrParseError", type_of(t));
        } else {
            panic!("Expected AddressError");
        }
    }
}