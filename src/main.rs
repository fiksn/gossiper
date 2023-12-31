// for now ignore everything
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
//#![allow(unreachable_code)]

mod addr;
mod dummy;
mod mutex;
mod resolve;
mod voter;

use addr::*;
use bitcoin::blockdata::constants::{genesis_block, ChainHash};
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::hash_types::{BlockHash, Txid};
use bitcoin::hashes::hex::FromHex;
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::PublicKey;
use bitcoin::Block;
use chrono::Utc;
use clap::Parser;
use dummy::*;
use futures::future::join;
use futures::future::join_all;
use futures::future::ready;
use lightning::chain::chaininterface::{BroadcasterInterface, ConfirmationTarget, FeeEstimator};
use lightning::chain::{chainmonitor, ChannelMonitorUpdateStatus};
use lightning::events::{MessageSendEvent, MessageSendEventsProvider};
use lightning::ln::channelmanager::{
    ChainParameters, ChannelManagerReadArgs, SimpleArcChannelManager,
};
use lightning::ln::features::{InitFeatures, NodeFeatures};
use lightning::ln::msgs::{self, RoutingMessageHandler};
use lightning::ln::peer_handler::{
    ErroringMessageHandler, IgnoringMessageHandler, PeerManager, SimpleArcPeerManager,
};
use lightning::log_info;
use lightning::routing::gossip::NodeId;
use lightning::routing::utxo::{UtxoLookup, UtxoLookupError, UtxoResult};
use lightning::sign::{EntropySource, InMemorySigner, KeysManager, SpendableOutputDescriptor};
use lightning::util::logger::{Level, Logger, Record};
use lightning_net_tokio::{setup_outbound, SocketDescriptor};
use lightning_persister::FilesystemPersister;
use rand::RngCore;
use rand::{thread_rng, Rng};
use resolve::*;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::string::ParseError;
use std::sync::{Arc, Mutex};
use std::{error::Error, net::SocketAddr};
use std::{
    thread,
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tokio::main;
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::time::timeout;
use voter::*;

struct ChannelInfo {
    node1: NodeId,
    node2: NodeId,
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum ParseThresholdError {
    #[error("Parse error")]
    ParseError,
    #[error("Parse int error {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Parse foat error {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Threshold {
    Number(u64),
    Percentage(f64),
}

impl FromStr for Threshold {
    type Err = ParseThresholdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with('%') {
            let x = &s[..s.len() - 1];
            let num = x.parse::<f64>()?;

            if num < 0f64 || num > 100f64 {
                return Err(ParseThresholdError::ParseError);
            }

            Ok(Self::Percentage(num))
        } else {
            let num = s.parse::<u64>()?;

            Ok(Self::Number(num))
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Nodes
    #[arg(short, long, num_args=1.., value_delimiter = ',',
    default_value = "03864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@3.33.236.230:9735",
    )]
    nodes: Vec<LightningNodeAddr>,

    /// Threshold
    #[arg(short, long, num_args = 1, default_value = "10%")]
    threshold: Threshold,
}

const DEBUG: bool = false;

#[main]
async fn main() {
    let args = Args::parse();

    // Init peripheral
    let logger = Arc::new(DummyLogger::new());
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let mut ephemeral_bytes = [0; 32];
    rand::thread_rng().fill_bytes(&mut ephemeral_bytes);

    let mut key = [0; 32];
    thread_rng().fill_bytes(&mut key);
    let keys_manager = Arc::new(KeysManager::new(
        &key,
        current_time.as_secs(),
        current_time.subsec_nanos(),
    ));

    //let persister = Arc::new(FilesystemPersister::new(".".to_string()));
    //let bitcoin = Arc::new(DummyBitcoin());

    let resolver: Arc<CachingChannelResolving<Arc<DummyLogger>>> =
        Arc::new(CachingChannelResolving::new(logger.clone()));

    let peer_manager: Arc<ResolvePeerManager> = Arc::new(PeerManager::new_routing_only(
        resolver.clone(),
        current_time.as_secs().try_into().unwrap(),
        &ephemeral_bytes,
        logger.clone(),
        keys_manager.clone(),
    ));

    resolver.register_peer_manager(peer_manager.clone());
    CachingChannelResolving::start(resolver.clone()).await;

    let voter = Arc::new(Voter::new(args.threshold, logger.clone()));
    voter.register_resolver(resolver.clone());
    resolver.register_voter(voter.clone());

    let mut futures: Vec<Box<dyn std::future::Future<Output = ()> + Unpin>> = Vec::new();

    for node in args.nodes.clone() {
        if let Some(future) = connect(node, peer_manager.clone()).await {
            futures.push(Box::new(Box::pin(future)));
        }
    }

    if DEBUG {
        let query = async {
            thread::sleep(Duration::from_secs(7));

            log_info!(logger, "Invoking query");

            let chanid = 869059488412139521u64;

            let nodeid1 = (*resolver)
                .get_node(
                    (*resolver)
                        .get_endpoints_async(chanid)
                        .await
                        .expect("channel data")
                        .nodes[0],
                )
                .unwrap()
                .node_id;
            let nodeid2 = (*resolver)
                .get_node(
                    (*resolver)
                        .get_endpoints_async(chanid)
                        .await
                        .expect("channel data")
                        .nodes[1],
                )
                .unwrap()
                .node_id;
            log_info!(logger, "{} --{}--> {}", nodeid1, chanid, nodeid2);
        };

        futures.push(Box::new(Box::pin(query)));
    }

    join_all(futures).await;
}

async fn connect(
    node: LightningNodeAddr,
    peer_manager: Arc<ResolvePeerManager>,
) -> Option<impl std::future::Future<Output = ()>> {
    let connect_timeout = Duration::from_secs(5);

    if let Ok(Ok(stream)) = timeout(connect_timeout, async {
        TcpStream::connect(node.endpoint)
            .await
            .map(|s| s.into_std().unwrap())
    })
    .await
    {
        println!("Connected to node {}", node);
        return Some(setup_outbound(
            peer_manager.clone(),
            node.node_id.as_pubkey().unwrap(),
            stream,
        ));
    } else {
        eprintln!("Failed to connect to the node {}", node);
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_threshold_parsing() {
        assert!(Threshold::from_str("100%") == Ok(Threshold::Percentage(100f64)));
        assert!(Threshold::from_str("99.9%") == Ok(Threshold::Percentage(99.9f64)));
        assert!(Threshold::from_str("101%").is_err());
        assert!(Threshold::from_str("%").is_err());
        assert!(Threshold::from_str("-1%").is_err());
        assert!(Threshold::from_str("aa").is_err());
        assert!(Threshold::from_str("%2").is_err());
        assert!(Threshold::from_str("21%") == Ok(Threshold::Percentage(21f64)));

        assert!(Threshold::from_str("-11").is_err());
        assert!(Threshold::from_str("21") == Ok(Threshold::Number(21)));
        assert!(Threshold::from_str("101") == Ok(Threshold::Number(101)));
    }
}
