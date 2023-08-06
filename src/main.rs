// for now ignore everything
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
//#![allow(unreachable_code)]

use lightning::ln::features::{InitFeatures, NodeFeatures};
use lightning::events::{MessageSendEvent, MessageSendEventsProvider};
use lightning::ln::msgs::RoutingMessageHandler;
use tokio::main;
use tokio::net::TcpStream;
use std::sync::Arc;
use lightning::ln::peer_handler::{IgnoringMessageHandler, ErroringMessageHandler, SimpleArcPeerManager, PeerManager};
use lightning::util::logger::{Logger, Record};
use chrono::Utc;
use rand::{thread_rng, Rng};
use std::time::{Duration, SystemTime};
use lightning_net_tokio::{SocketDescriptor, setup_outbound};
use lightning::sign::{EntropySource, InMemorySigner, KeysManager, SpendableOutputDescriptor};
use lightning::chain::{chainmonitor, ChannelMonitorUpdateStatus};
use lightning_persister::FilesystemPersister;
use lightning::chain::chaininterface::{BroadcasterInterface, ConfirmationTarget, FeeEstimator};
use bitcoin::blockdata::transaction::Transaction;
use lightning::routing::utxo::{UtxoLookup, UtxoResult, UtxoLookupError};
use bitcoin::hash_types::{BlockHash, Txid};
use lightning::ln::channelmanager::{
	ChainParameters, ChannelManagerReadArgs, SimpleArcChannelManager,
};
use bitcoin::secp256k1::PublicKey;
use std::str::FromStr;
use std::thread;

use tokio::time::timeout;
use tokio::net::ToSocketAddrs;

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


struct DummyHandler();

impl MessageSendEventsProvider for DummyHandler {
	fn get_and_clear_pending_msg_events(&self) -> Vec<MessageSendEvent> {
        Vec::new()
    }
}

impl RoutingMessageHandler for DummyHandler {
    fn handle_node_announcement(&self, msg: &lightning::ln::msgs::NodeAnnouncement) -> Result<bool, lightning::ln::msgs::LightningError> {
        Ok(false)
    }

    fn handle_channel_announcement(&self, msg: &lightning::ln::msgs::ChannelAnnouncement) -> Result<bool, lightning::ln::msgs::LightningError> {
        Ok(false)
    }

    fn handle_channel_update(&self, msg: &lightning::ln::msgs::ChannelUpdate) -> Result<bool, lightning::ln::msgs::LightningError> {
        println!("CU {:?}", msg);
        Ok(false)
    }

    fn get_next_channel_announcement(&self, starting_point: u64) -> Option<(lightning::ln::msgs::ChannelAnnouncement, Option<lightning::ln::msgs::ChannelUpdate>, Option<lightning::ln::msgs::ChannelUpdate>)> {
        None
    }

    fn get_next_node_announcement(&self, starting_point: Option<&lightning::routing::gossip::NodeId>) -> Option<lightning::ln::msgs::NodeAnnouncement> {
        None
    }

    fn peer_connected(&self, their_node_id: &PublicKey, init: &lightning::ln::msgs::Init, inbound: bool) -> Result<(), ()> {
        Ok(())
    }

    fn handle_reply_channel_range(&self, their_node_id: &PublicKey, msg: lightning::ln::msgs::ReplyChannelRange) -> Result<(), lightning::ln::msgs::LightningError> {
        Ok(())
    }

    fn handle_reply_short_channel_ids_end(&self, their_node_id: &PublicKey, msg: lightning::ln::msgs::ReplyShortChannelIdsEnd) -> Result<(), lightning::ln::msgs::LightningError> {
        Ok(())
    }

    fn handle_query_channel_range(&self, their_node_id: &PublicKey, msg: lightning::ln::msgs::QueryChannelRange) -> Result<(), lightning::ln::msgs::LightningError> {
        Ok(())
    }

    fn handle_query_short_channel_ids(&self, their_node_id: &PublicKey, msg: lightning::ln::msgs::QueryShortChannelIds) -> Result<(), lightning::ln::msgs::LightningError> {
        Ok(())
    }

    fn processing_queue_high(&self) -> bool {
        false
    }

    fn provided_node_features(&self) -> lightning::ln::features::NodeFeatures {
        NodeFeatures::empty()
    }

    fn provided_init_features(&self, their_node_id: &PublicKey) -> lightning::ln::features::InitFeatures {
        InitFeatures::empty()
    }
}

type DummyPeerManager = PeerManager<SocketDescriptor, ErroringMessageHandler, IgnoringMessageHandler, IgnoringMessageHandler, Arc<DummyLogger>, IgnoringMessageHandler, Arc<KeysManager>>;

#[main]
async fn main() {
    // Init peripheral
    let logger = Arc::new(DummyLogger());
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    let mut ephemeral_bytes = [0; 32];
	rand::thread_rng().fill_bytes(&mut ephemeral_bytes);

	let mut key = [0; 32];
	thread_rng().fill_bytes(&mut key);
    let keys_manager = Arc::new(KeysManager::new(&key, current_time.as_secs(), current_time.subsec_nanos()));
    
    //let persister = Arc::new(FilesystemPersister::new(".".to_string()));
    //let bitcoin = Arc::new(DummyBitcoin());
   
    let peer_manager : Arc<DummyPeerManager> = Arc::new(PeerManager::new_routing_only(
        IgnoringMessageHandler{}, // todo
        current_time.as_secs().try_into().unwrap(),
        &ephemeral_bytes,
		logger.clone(),
        keys_manager.clone()
    ));

    let pubkey: PublicKey = PublicKey::from_str("0327f763c849bfd218910e41eef74f5a737989358ab3565f185e1a61bb7df445b8").unwrap();
    let server_details = "89.212.253.230:9735";
    let connect_timeout = Duration::from_secs(5);

    match timeout(connect_timeout, TcpStream::connect(server_details)).await {
        Ok(stream) => {
            println!("Connected to the server on {}", server_details);

            let c = setup_outbound(peer_manager, pubkey, stream.unwrap().into_std().unwrap()).await;
            println!("{:?}", c)
        }
        Err(e) => {
            eprintln!("Failed to connect to the server: {}", e);
        }
    }
}
