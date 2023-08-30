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
use lightning::util::logger::{Logger, Level, Record};
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

/// Dummy implementations

pub struct DummyLogger{
    level: Level,
}

impl DummyLogger {
    pub fn new() -> DummyLogger {
        DummyLogger {
            level: Level::Trace,
        }
    }
}

impl Logger for DummyLogger {
    fn log(&self, record: &Record) {
        let raw_log = record.args.to_string();
        if record.level >= self.level {
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