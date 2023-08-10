// for now ignore everything
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
//#![allow(unreachable_code)]

use bitcoin::Block;
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::hash_types::{BlockHash, Txid};
use bitcoin::hashes::hex::FromHex;
use bitcoin::secp256k1::PublicKey;
use chrono::Utc;
use lightning::chain::chaininterface::{BroadcasterInterface, ConfirmationTarget, FeeEstimator};
use lightning::chain::{chainmonitor, ChannelMonitorUpdateStatus};
use lightning::events::{MessageSendEvent, MessageSendEventsProvider};
use lightning::ln::channelmanager::{
    ChainParameters, ChannelManagerReadArgs, SimpleArcChannelManager,
};
use lightning::ln::features::{InitFeatures, NodeFeatures};
use lightning::ln::msgs::{RoutingMessageHandler, self};
use lightning::ln::peer_handler::{
    ErroringMessageHandler, IgnoringMessageHandler, PeerManager, SimpleArcPeerManager,
};
use lightning::routing::gossip::NodeId;
use lightning::routing::utxo::{UtxoLookup, UtxoLookupError, UtxoResult};
use lightning::sign::{EntropySource, InMemorySigner, KeysManager, SpendableOutputDescriptor};
use lightning::util::logger::{Logger, Record};
use lightning_net_tokio::{setup_outbound, SocketDescriptor};
use lightning_persister::FilesystemPersister;
use bitcoin::blockdata::constants::{genesis_block, ChainHash};
use bitcoin::network::constants::Network;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use rand::RngCore;
use std::time::{Duration, SystemTime};
use tokio::main;
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::time::timeout;
use futures::future::join;
use std::ops::Deref;
use std::{error::Error, net::SocketAddr};
use std::fmt;
use clap::Parser;

struct DummyLogger();
impl Logger for DummyLogger {
    fn log(&self, record: &Record) {
        let raw_log = record.args.to_string();
        println!(
            "{} {:<5} [{}:{}] {}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            record.level.to_string(),
            record.module_path,
            record.line,
            raw_log
        );
    }
}

struct DummyBitcoin();
impl BroadcasterInterface for DummyBitcoin {
    fn broadcast_transactions(&self, txs: &[&Transaction]) {
        // do nothing
    }
}
impl UtxoLookup for DummyBitcoin {
    fn get_utxo(&self, _genesis_hash: &BlockHash, _short_channel_id: u64) -> UtxoResult {
        UtxoResult::Sync(Err(UtxoLookupError::UnknownChain))
    }
}
impl FeeEstimator for DummyBitcoin {
    fn get_est_sat_per_1000_weight(&self, confirmation_target: ConfirmationTarget) -> u32 {
        0
    }
}

/*
pub struct QueryShortChannelIds {
    pub chain_hash: BlockHash,
    pub short_channel_ids: Vec<u64>,
}

peer_manager.node_id_to_descriptor
 lightning::ln::peer_handler::SocketDescriptor
 send_data(&mut self, data: &[u8], resume_read: bool) -> usize


 if msg.ponglen < 65532 {
					let resp = msgs::Pong { byteslen: msg.ponglen };
					self.enqueue_message(&mut *peer_mutex.lock().unwrap(), &resp);
				}


*/

struct ChannelInfo {
    node1: NodeId,
    node2: NodeId,
}

struct DummyHandler {
    info: Mutex<HashMap<u64, ChannelInfo>>,
    peer_manager: Option<Arc<DummyPeerManager>>,
}

impl MessageSendEventsProvider for DummyHandler {
    fn get_and_clear_pending_msg_events(&self) -> Vec<MessageSendEvent> {
        Vec::new()
    }
}

impl RoutingMessageHandler for DummyHandler {
    fn handle_node_announcement(
        &self,
        msg: &lightning::ln::msgs::NodeAnnouncement,
    ) -> Result<bool, lightning::ln::msgs::LightningError> {
        println!("{:?}", msg.contents);
        Ok(false)
    }

    fn handle_channel_announcement(
        &self,
        msg: &lightning::ln::msgs::ChannelAnnouncement,
    ) -> Result<bool, lightning::ln::msgs::LightningError> {
        /*
        {
            let mut guard = self.info.lock().unwrap();
            guard.insert(msg.contents.short_channel_id, ChannelInfo { node1: msg.contents.node_id_1, node2: msg.contents.node_id_2});
        }
        */
        println!("{:?}", msg.contents);
        Ok(false)
    }

    fn handle_channel_update(
        &self,
        msg: &lightning::ln::msgs::ChannelUpdate,
    ) -> Result<bool, lightning::ln::msgs::LightningError> {
        println!(
            "Chan {} {} {}",
            msg.contents.short_channel_id, msg.contents.flags, msg.contents.chain_hash
        );

        // flags bit 0 direction, bit 1 disable
        if msg.contents.flags & 0x2 == 0x2 {
            println!("Disable!! {}", msg.contents.short_channel_id)
        }

        Ok(false)
    }

    fn get_next_channel_announcement(
        &self,
        starting_point: u64,
    ) -> Option<(
        lightning::ln::msgs::ChannelAnnouncement,
        Option<lightning::ln::msgs::ChannelUpdate>,
        Option<lightning::ln::msgs::ChannelUpdate>,
    )> {
        None
    }

    fn get_next_node_announcement(
        &self,
        starting_point: Option<&lightning::routing::gossip::NodeId>,
    ) -> Option<lightning::ln::msgs::NodeAnnouncement> {
        None
    }

    fn peer_connected(
        &self,
        their_node_id: &PublicKey,
        init: &lightning::ln::msgs::Init,
        inbound: bool,
    ) -> Result<(), ()> {
        Ok(())
    }

    fn handle_reply_channel_range(
        &self,
        their_node_id: &PublicKey,
        msg: lightning::ln::msgs::ReplyChannelRange,
    ) -> Result<(), lightning::ln::msgs::LightningError> {
        println!("CHRANGE END");
        Ok(())
    }

    fn handle_reply_short_channel_ids_end(
        &self,
        their_node_id: &PublicKey,
        msg: lightning::ln::msgs::ReplyShortChannelIdsEnd,
    ) -> Result<(), lightning::ln::msgs::LightningError> {
        println!("CHID END");
        Ok(())
    }

    fn handle_query_channel_range(
        &self,
        their_node_id: &PublicKey,
        msg: lightning::ln::msgs::QueryChannelRange,
    ) -> Result<(), lightning::ln::msgs::LightningError> {
        Ok(())
    }

    fn handle_query_short_channel_ids(
        &self,
        their_node_id: &PublicKey,
        msg: lightning::ln::msgs::QueryShortChannelIds,
    ) -> Result<(), lightning::ln::msgs::LightningError> {
        Ok(())
    }

    fn processing_queue_high(&self) -> bool {
        false
    }

    fn provided_node_features(&self) -> lightning::ln::features::NodeFeatures {
        NodeFeatures::empty()
    }

    fn provided_init_features(
        &self,
        their_node_id: &PublicKey,
    ) -> lightning::ln::features::InitFeatures {
        let mut features = InitFeatures::empty();
		
        features.set_data_loss_protect_optional();
		features.set_upfront_shutdown_script_optional();
		features.set_variable_length_onion_optional();
		features.set_static_remote_key_optional();
		features.set_payment_secret_optional();
		features.set_basic_mpp_optional();
		features.set_wumbo_optional();
		features.set_shutdown_any_segwit_optional();
		features.set_channel_type_optional();
		features.set_scid_privacy_optional();
		features.set_zero_conf_optional();
        features.set_gossip_queries_optional();
        
        features
    }
}

type DummyPeerManager = PeerManager<
    SocketDescriptor,
    ErroringMessageHandler,
    Arc<DummyHandler>,
    IgnoringMessageHandler,
    Arc<DummyLogger>,
    IgnoringMessageHandler,
    Arc<KeysManager>,
>;


///////


/// Obtain gossip message from lightning network
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server
    #[arg(short, long)]
    server: u16
}

/////


#[main]
async fn main() {
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

    let handler = Arc::new(DummyHandler { info: Mutex::new(HashMap::new()), peer_manager: None });
    
    let peer_manager: Arc<DummyPeerManager> = Arc::new(PeerManager::new_routing_only(
        handler,
        current_time.as_secs().try_into().unwrap(),
        &ephemeral_bytes,
        logger.clone(),
        keys_manager.clone(),
    ));

    //handler.peer_manager = Some(peer_manager.clone());
    
    //let args = Args::parse();

    //peer_manager.get_peer_node_ids().
    //["0327f763c849bfd218910e41eef74f5a737989358ab3565f185e1a61bb7df445b8"].unwrap();
    // 03864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f@3.33.236.230:9735
    //024271a1be2d7a3e2a276b241257be734d843885d252f50575e4c7db2691aedd3a@81.56.42.102:18000
    let pubkey: PublicKey =
        PublicKey::from_str("0327f763c849bfd218910e41eef74f5a737989358ab3565f185e1a61bb7df445b8")
            .unwrap();
    let server_details = "89.212.253.230:9735";
    let connect_timeout = Duration::from_secs(5);

    match timeout(connect_timeout, TcpStream::connect(server_details)).await {
        Ok(stream) => {
            println!("Connected to the server on {}", server_details);

            let future1 = setup_outbound(peer_manager.clone(), pubkey, stream.unwrap().into_std().unwrap());

            let future2 = async {
                thread::sleep(Duration::from_secs(5));

                println!("Sending to random node");
                
                peer_manager.clone().send_to_random_node(&msgs::QueryShortChannelIds { 
                    //chain_hash: genesis_block(Network::Bitcoin).header.block_hash(),
                    chain_hash: BlockHash::from_hex("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f").unwrap(),
                    short_channel_ids: vec![869059488412139521]} );

                peer_manager.clone().send_to_random_node(&msgs::QueryShortChannelIds { 
                    //chain_hash: genesis_block(Network::Bitcoin).header.block_hash(),
                    chain_hash: BlockHash::from_hex("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f").unwrap(),
                    short_channel_ids: vec![874232690414845953]} );
                
                peer_manager.clone().send_to_random_node(&msgs::Ping{ ponglen: 1337,  byteslen: 1336 });
                println!("...Done");
            };
        
            join(future1, future2).await;
        }
        Err(e) => {
            eprintln!("Failed to connect to the server: {}", e);
        }
    }
}
