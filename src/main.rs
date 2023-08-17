// for now ignore everything
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
//#![allow(unreachable_code)]

mod dummy;
mod addr;
mod resolve;

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
use lightning::routing::gossip::NodeId;
use lightning::routing::utxo::{UtxoLookup, UtxoLookupError, UtxoResult};
use lightning::sign::{EntropySource, InMemorySigner, KeysManager, SpendableOutputDescriptor};
use lightning::util::logger::{Logger, Record};
use lightning_net_tokio::{setup_outbound, SocketDescriptor};
use lightning_persister::FilesystemPersister;
use rand::RngCore;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::{thread, time::{Duration, SystemTime}};
use std::{error::Error, net::SocketAddr};
use tokio::main;
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::time::timeout;
use futures::future::ready;

struct ChannelInfo {
    node1: NodeId,
    node2: NodeId,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Nodes
    #[arg(short, long, num_args=1.., value_delimiter = ',',
    default_value = "03864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@3.33.236.230:9735",
    )]
    nodes: Vec<LightningNodeAddr>,
}

#[main]
async fn main() {
    let args = Args::parse();

    // Init peripheral
    let logger = Arc::new(DummyLogger());
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

    let handler = Arc::new(DummyHandler {
        info: Mutex::new(HashMap::new()),
        peer_manager: Mutex::new(None),
    });

    let peer_manager: Arc<DummyPeerManager> = Arc::new(PeerManager::new_routing_only(
        handler.clone(),
        current_time.as_secs().try_into().unwrap(),
        &ephemeral_bytes,
        logger.clone(),
        keys_manager.clone(),
    ));

    *(handler.peer_manager.lock().unwrap()) = Some(peer_manager.clone());

   

    /*
    let connect_timeout = Duration::from_secs(5);

   
    match timeout(
        connect_timeout,
        TcpStream::connect(args.nodes.clone().get(0).unwrap().endpoint),
    )
    .await
    {
        Ok(stream) => {
            println!("Connected to node {}", args.nodes.clone().get(0).unwrap());

            let future1 = setup_outbound(
                peer_manager.clone(),
                args.nodes
                    .clone()
                    .get(0)
                    .unwrap()
                    .node_id
                    .as_pubkey()
                    .unwrap(),
                stream.unwrap().into_std().unwrap(),
            );

            let future2 = async {
                thread::sleep(Duration::from_secs(5));

                println!("Sending to random node");

                peer_manager
                    .clone()
                    .send_to_random_node(&msgs::QueryShortChannelIds {
                        //chain_hash: genesis_block(Network::Bitcoin).header.block_hash(),
                        chain_hash: BlockHash::from_hex(
                            "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce27f",
                        )
                        .unwrap(),
                        short_channel_ids: vec![869059488412139521],
                    });

                peer_manager
                    .clone()
                    .send_to_random_node(&msgs::QueryShortChannelIds {
                        //chain_hash: genesis_block(Network::Bitcoin).header.block_hash(),
                        chain_hash: BlockHash::from_hex(
                            "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce27f",
                        )
                        .unwrap(),
                        short_channel_ids: vec![874232690414845953],
                    });

                peer_manager.clone().send_to_random_node(&msgs::Ping {
                    ponglen: 1337,
                    byteslen: 1336,
                });
                println!("...Done");
            };

            join(future1, future2).await;
            println!("AWAITED");
        }
        Err(e) => {
            eprintln!("Failed to connect to the server: {}", e);
        }
    }

    println!("Demo");
    */

    let mut futures: Vec<Box<dyn std::future::Future<Output = ()> + Unpin>> = Vec::new();

    for node in args.nodes.clone() {
        if let Some(future) = connect(node, peer_manager.clone()).await {
            println!("Awaiting...");
            futures.push(Box::new(Box::pin(future)));
        }
    }

    join_all(futures).await;
}

async fn connect(node: LightningNodeAddr, peer_manager: Arc<DummyPeerManager>) -> Option<impl std::future::Future<Output=()>> {
    let connect_timeout = Duration::from_secs(5);

    if let Ok(Ok(stream)) = timeout(connect_timeout, async { TcpStream::connect(node.endpoint).await.map(|s| s.into_std().unwrap()) }).await {
        println!("Connected to node {}", node);
		return Some(setup_outbound(
            peer_manager.clone(),
            node.node_id.as_pubkey().unwrap(),
            stream,
        ));
	} else { 
        eprintln!("Failed to connect to the server");
        return None;
    }
}
