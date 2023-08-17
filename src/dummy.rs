use bitcoin::blockdata::constants::{genesis_block, ChainHash};
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::hash_types::{BlockHash, Txid};
use bitcoin::hashes::hex::FromHex;
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::PublicKey;
use bitcoin::Block;
use chrono::Utc;
use clap::Parser;
use futures::future::join;
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
use std::thread;
use std::time::{Duration, SystemTime};
use std::{error::Error, net::SocketAddr};
use tokio::main;
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::time::timeout;
use lightning::routing::gossip::ChannelInfo;

pub type DummyPeerManager = PeerManager<
    SocketDescriptor,
    ErroringMessageHandler,
    Arc<DummyHandler>,
    IgnoringMessageHandler,
    Arc<DummyLogger>,
    IgnoringMessageHandler,
    Arc<KeysManager>,
>;

pub struct DummyLogger();
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

pub struct DummyBitcoin();
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

pub struct DummyHandler {
    pub info: Mutex<HashMap<u64, ChannelInfo>>,
    pub peer_manager: Mutex<Option<Arc<DummyPeerManager>>>,
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
         
        if let Ok(pm) = self.peer_manager.try_lock() {
            println!("PM");
            pm.clone().unwrap().send_to_random_node(&msgs::Ping {
                ponglen: 1335,
                byteslen: 1335,
            });
        } else {
            // The lock was not acquired because another thread holds it.
            println!("Mutex is currently locked by another thread.");
        }

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
        features.set_gossip_queries_optional(); // this is needed for LND which won't create GossipSyncer

        features
    }
}

