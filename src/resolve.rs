use futures::future::Future;
use futures::channel::oneshot;
use futures::channel::oneshot::*;
use futures::executor::block_on;
use lightning::routing::gossip::NodeId;

use std::sync::Mutex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use bitcoin::blockdata::constants::genesis_block;
use bitcoin::network::constants::Network;

use lightning_net_tokio::{setup_outbound, SocketDescriptor};
use lightning::ln::peer_handler::{
    ErroringMessageHandler, IgnoringMessageHandler, PeerManager, SimpleArcPeerManager,
};
use lightning::ln::features::{InitFeatures, NodeFeatures};

use lightning::sign::KeysManager;
use lightning::events::MessageSendEvent;
use lightning::ln::msgs::{self, RoutingMessageHandler};
use lightning::events::MessageSendEventsProvider;
use bitcoin::secp256k1::PublicKey;
use tokio::time;
use std::time::{Duration, SystemTime};


use super::dummy::*;
use super::mutex::*;
use super::voter::*;
use parking_lot::lock_api::RawMutex;

pub type ResolvePeerManager = PeerManager<
    SocketDescriptor,
    ErroringMessageHandler,
    Arc<CachingChannelResolving>,
    IgnoringMessageHandler,
    Arc<DummyLogger>,
    IgnoringMessageHandler,
    Arc<KeysManager>,
>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct EndpointData {
	pub short_channel_id: u64,
	pub nodes: [NodeId; 2],
}

unsafe impl Send for EndpointData {}


#[derive(Debug)]
struct Pending {
	id: u64,
	sender: Sender<EndpointData>,
}

pub trait ChannelResolving {
	fn get_endpoints_async(&self, id: u64) -> Pin<Box<dyn Future<Output = Result<EndpointData, Canceled>> + Send>>;
    fn is_endpoint_cached(&self, id: u64) -> bool;
    fn get_node(&self, node_id: NodeId) -> Option<lightning::ln::msgs::UnsignedNodeAnnouncement>;
}

pub struct CachingChannelResolving {
	chan_id_cache: Mutex<HashMap<u64, EndpointData>>,
	node_cache: Mutex<HashMap<NodeId, lightning::ln::msgs::UnsignedNodeAnnouncement>>,
	pending: Mutex<Vec<Pending>>,
	peer_manager: Mutex<Option<Arc<ResolvePeerManager>>>,
    voter: Mutex<Option<Arc<Voter>>>,
	server_lock: RMutexMax,
}

impl CachingChannelResolving {
	pub fn new() -> CachingChannelResolving {
		CachingChannelResolving{
			chan_id_cache: Mutex::new(HashMap::new()),
			node_cache: Mutex::new(HashMap::new()),
			pending: Mutex::new(Vec::new()),
			peer_manager: Mutex::new(None),
            voter: Mutex::new(None),
			server_lock: RMutexMax::INIT,
		}
	}

	pub fn register_peer_manager(&self, peer_manager: Arc<ResolvePeerManager>) {
		*(self.peer_manager.lock().unwrap()) = Some(peer_manager.clone());
	}

    pub fn register_voter(&self, voter: Arc<Voter>) {
		*(self.voter.lock().unwrap()) = Some(voter.clone());
	}

    // TODO: when this is a method this does not work due to lifetimes
	pub async fn start(other: Arc<Self>) {
		tokio::spawn(async move {
			let mut interval_stream = time::interval(Duration::from_secs(10));
					
        	loop {
				other.clone().timer_func();
	        	interval_stream.tick().await;
        	}
		});
	}

    fn timer_func(&self) {
		let todo = self.get_todo();
		if todo.is_empty() {
			self.resolve();
		} else {
			if let Ok(pm) = self.peer_manager.try_lock() {
				let vec: Vec<_> = todo.into_iter().collect();
				if self.server_lock.try_lock_max(Duration::from_secs(128)) {
					pm.clone().unwrap().send_to_random_node(&msgs::QueryShortChannelIds {
						chain_hash: genesis_block(Network::Bitcoin).header.block_hash(),
						short_channel_ids: vec,
					});
				} else {
					println!("Did not get response from server yet");
				}
			}
		}
	}

	fn get_todo(&self) -> HashSet::<u64> {
		let mut set = HashSet::<u64>::new();

		let data = self.pending.lock().unwrap();
		for one in &*data {
			if self.chan_id_cache.lock().unwrap().get(&one.id) == None {
				set.insert(one.id);
			}
		}
		
		set
	}
	
	fn resolve(&self) {
		let mut data = self.pending.lock().unwrap();
		
		while let Some(e) = data.pop() {
			if let Some(num) = self.chan_id_cache.lock().unwrap().get(&e.id) {
				e.sender.send(*num);
			}
		}
	}
}

impl ChannelResolving for CachingChannelResolving {
	fn get_endpoints_async(&self, id: u64) -> Pin<Box<dyn Future<Output = Result<EndpointData, Canceled>> + Send>> {
		let (sender, receiver) = oneshot::channel::<EndpointData>();

		self.pending.lock().unwrap().push(Pending { id: id, sender: sender});
		Box::pin(receiver)
	}

    fn get_node(&self, node_id: NodeId) -> Option<lightning::ln::msgs::UnsignedNodeAnnouncement> {
		let guard = self.node_cache.lock().unwrap();
		return guard.get(&node_id).cloned();
	}

    fn is_endpoint_cached(&self, id: u64) -> bool {
        self.chan_id_cache.lock().unwrap().get(&id) != None
    }
}

impl MessageSendEventsProvider for CachingChannelResolving {
    fn get_and_clear_pending_msg_events(&self) -> Vec<MessageSendEvent> {
        Vec::new()
    }
}

impl RoutingMessageHandler for CachingChannelResolving {
    fn handle_node_announcement(
        &self,
        msg: &lightning::ln::msgs::NodeAnnouncement,
    ) -> Result<bool, lightning::ln::msgs::LightningError> {
		self.node_cache.lock().unwrap().insert(msg.contents.node_id, msg.contents.clone());
        Ok(false)
    }

    fn handle_channel_announcement(
        &self,
        msg: &lightning::ln::msgs::ChannelAnnouncement,
    ) -> Result<bool, lightning::ln::msgs::LightningError> {
        self.chan_id_cache.lock().unwrap().insert(msg.contents.short_channel_id, EndpointData {
			short_channel_id: msg.contents.short_channel_id,
			nodes: [ msg.contents.node_id_1, msg.contents.node_id_2],
		});
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

        // flags: bit 0 direction, bit 1 disable
        let direction = (msg.contents.flags & 0x1) as usize;
        let chanid = msg.contents.short_channel_id; 
        let voter = self.voter.lock().unwrap().clone().unwrap();

        if msg.contents.flags & 0x2 == 0x2 {
            // Disable
            println!("DISABLE direction: {}", direction);
            
            tokio::spawn(async move {
                voter.burek().await;
            });

            /*
            tokio::spawn(async move {
                    //let node = ss.get_node(block_on(ss.get_endpoints_async(chanid)).expect("channel data").nodes[direction]).unwrap();
                    let s = Arc::new(Mutex::new(self));
                    println!("Disable!!" );
                    CachingChannelResolving::burek(s.clone());
            });
            */
            
            return Ok(false);
        } else {
            // Enable
            if !self.is_endpoint_cached(chanid) {
                // Skip further processing
                return Ok(false);
            }

            let node = self.get_node(block_on(self.get_endpoints_async(chanid)).expect("channel data").nodes[direction]).unwrap();
            println!("Enable!! {} {}", chanid, node.node_id);
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

        // When gossip queries are negotiated you MUST send GossipTimestampFilter or else no gossip message will be received
        if let Ok(pm) = self.peer_manager.try_lock() {
            let peer = pm.clone().unwrap();
            let node = their_node_id.clone();
            let current_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap();

            // Spawn this in new thread to avoid lock being held
            tokio::spawn(async move {
                let mut wait = time::interval(Duration::from_secs(1));
                wait.tick().await;

                peer.send_to_node(node, &msgs::GossipTimestampFilter {
                        chain_hash: genesis_block(Network::Bitcoin).header.block_hash(),
                        first_timestamp: current_time.as_secs() as u32,
                        timestamp_range: u32::MAX,
                });
            });
        }

        Ok(())
    }

    fn handle_reply_channel_range(
        &self,
        their_node_id: &PublicKey,
        msg: lightning::ln::msgs::ReplyChannelRange,
    ) -> Result<(), lightning::ln::msgs::LightningError> {
        Ok(())
    }

    fn handle_reply_short_channel_ids_end(
        &self,
        their_node_id: &PublicKey,
        msg: lightning::ln::msgs::ReplyShortChannelIdsEnd,
    ) -> Result<(), lightning::ln::msgs::LightningError> {
		// Unlock the raw mutex
		unsafe {
			if self.server_lock.is_locked() {
				self.server_lock.unlock();
			}
		}
		self.timer_func();
	
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
        features.set_gossip_queries_optional(); // this is needed for LND which won't create GossipSyncer else

        features
    }
}
  
#[cfg(test)]
mod tests {
    use super::*;
	use tokio::test;

    //#[test]
	#[tokio::test]
    async fn test_resolve() {
		let mut resolver = CachingChannelResolving::new(None);

		let a = resolver.get_endpoints_async(1);
		resolver.resolve();
		
		let result = a.await;

		println!("{:?}", result);

		panic!("Foo");
    }
}