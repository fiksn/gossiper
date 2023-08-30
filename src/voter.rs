use std::sync::{Arc, Mutex};
use std::ops::Deref;
use lightning::util::logger::Logger;
use lightning::*;
use std::collections::{HashMap, HashSet};
use lightning::routing::gossip::NodeId;

use super::resolve::*;
use super::dummy::*;

/// Voter is able to count votes regarding unavailability

pub struct Voter<L: Deref + Send + std::marker::Sync + 'static> where L::Target: Logger {
    logger: L,
    threshold: u8,
    resolver: Mutex<Option<Arc<CachingChannelResolving<Arc<DummyLogger>>>>>,
    data: Mutex<HashMap<NodeId, HashSet<u64>>>,
}
impl <L: Deref + Send + std::marker::Sync + 'static> Voter<L> where L::Target: Logger {
	pub fn new(threshold: u8, logger: L) -> Voter<L> {
		Voter{
            resolver: Mutex::new(None),
            logger: logger,
            threshold: threshold,
            data: Mutex::new(HashMap::new()),
        }
    }

    pub fn register_resolver(&self, resolver: Arc<CachingChannelResolving<Arc<DummyLogger>>>) {
		*(self.resolver.lock().unwrap()) = Some(resolver.clone());
	}

    pub async fn disable(&self, chanid: u64, direction: usize) {
        let res = self.resolver.lock().unwrap().clone().unwrap();
        let node = res.get_node(res.get_endpoints_async(chanid).await.expect("channel data").nodes[direction]).expect("node data");

        log_trace!(self.logger, "DISABLE chid: {} direction: {} node: {} alias: {}", chanid, direction, node.node_id, node.alias);
        let guard = self.data.lock().unwrap();

        
    }

    pub async fn enable(&self, chanid: u64, direction: usize) {
        let res = self.resolver.lock().unwrap().clone().unwrap();

        if !res.is_endpoint_cached(chanid) {
            // Ignore enabling channels which we are unaware of
            return;
        }

        let node = res.get_node(res.get_endpoints_async(chanid).await.expect("channel data").nodes[direction]).expect("node data");

        log_trace!(self.logger, "ENABLE chid: {} direction: {} node: {} alias: {}", chanid, direction, node.node_id, node.alias);
    }

}


