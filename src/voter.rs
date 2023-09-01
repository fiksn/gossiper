use lightning::routing::gossip::{NodeAlias, NodeId};
use lightning::util::logger::Logger;
use lightning::*;
use reqwest;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use super::dummy::*;
use super::resolve::*;

/// Voter is able to count votes regarding unavailability

pub struct NodeData {
    pub node_id: NodeId,
    pub alias: NodeAlias,
}

pub struct Voter<L: Deref + Send + std::marker::Sync + 'static>
where
    L::Target: Logger,
{
    logger: L,
    threshold: u8,
    resolver: Mutex<Option<Arc<CachingChannelResolving<Arc<DummyLogger>>>>>,
    votes: Mutex<HashMap<NodeId, HashSet<u64>>>,
}
impl<L: Deref + Send + std::marker::Sync + 'static> Voter<L>
where
    L::Target: Logger,
{
    pub fn new(threshold: u8, logger: L) -> Voter<L> {
        Voter {
            resolver: Mutex::new(None),
            logger: logger,
            threshold: threshold,
            votes: Mutex::new(HashMap::new()),
        }
    }

    pub fn register_resolver(&self, resolver: Arc<CachingChannelResolving<Arc<DummyLogger>>>) {
        *(self.resolver.lock().unwrap()) = Some(resolver.clone());
    }

    async fn get_node(&self, chanid: u64, direction: usize) -> NodeData {
        let res = self.resolver.lock().unwrap().clone().unwrap();

        let id = res
            .get_endpoints_async(chanid)
            .await
            .expect("channel data")
            .nodes[direction];

        let n = res.get_node(id);

        if n == None {
            log_trace!(self.logger, "Node data for nodeid {} not available", id);

            let result: [u8; 32] = [0; 32];

            return NodeData {
                node_id: id,
                alias: NodeAlias(result),
            };
        }

        let node = n.unwrap();

        NodeData {
            node_id: node.node_id,
            alias: node.alias,
        }
    }

    pub async fn disable(&self, chanid: u64, direction: usize) {
        let res = self.resolver.lock().unwrap().clone().unwrap();
        let node = self.get_node(chanid, direction).await;

        log_trace!(
            self.logger,
            "DISABLE chid: {} direction: {} node: {} alias: {}",
            chanid,
            direction,
            node.node_id,
            node.alias
        );
        let num: u8;

        {
            let mut guard = self.votes.lock().unwrap();

            if let Some(one) = guard.get_mut(&node.node_id) {
                (*one).insert(chanid);
                num = (*one).len() as u8;
            } else {
                let mut one = HashSet::new();
                one.insert(chanid);
                guard.insert(node.node_id, one);
                num = 1;
            }
        }

        if num >= self.threshold {
            let info = Self::get_nodeinfo(node.node_id).await;
            log_info!(
                self.logger,
                "THRESHOLD BREACHED num: {}/{} node: {} alias: {}",
                num,
                info.map_or(0, |info| info.channelcount),
                node.node_id,
                node.alias
            );
        }
    }

    pub async fn enable(&self, chanid: u64, direction: usize) {
        {
            let res = self.resolver.lock().unwrap().clone().unwrap();

            if !res.is_endpoint_cached(chanid) {
                // Ignore enabling channels which we are unaware of
                return;
            }
        }

        let node = self.get_node(chanid, direction).await;

        log_trace!(
            self.logger,
            "ENABLE chid: {} direction: {} node: {} alias: {}",
            chanid,
            direction,
            node.node_id,
            node.alias
        );

        let mut guard = self.votes.lock().unwrap();
        if let Some(one) = guard.get(&node.node_id) {
            if one.len() as u8 >= self.threshold {
                log_info!(
                    self.logger,
                    "THRESHOLD NOT BREACHED anymore, node: {} alias: {}",
                    node.node_id,
                    node.alias
                );
            }
        }
        guard.remove(&node.node_id);
    }

    async fn get_nodeinfo(node_id: NodeId) -> Option<NodeInfo> {
        reqwest::get(format!("https://1ml.com/node/{}/json", node_id.to_string()))
            .await
            .ok()?
            .json::<NodeInfo>()
            .await
            .ok()
    }
}

#[derive(Debug, Deserialize)]
pub struct NodeInfo {
    pub pub_key: String,
    pub channelcount: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::PublicKey;
    use tokio::test;

    #[tokio::test]
    async fn test_get_nodeinfo() {
        let id: &str = "0327f763c849bfd218910e41eef74f5a737989358ab3565f185e1a61bb7df445b8";
        if let Some(nodeinfo) = Voter::<Arc<DummyLogger>>::get_nodeinfo(NodeId::from_pubkey(
            &PublicKey::from_str(id).unwrap(),
        ))
        .await
        {
            assert_eq!(id, nodeinfo.pub_key);
            assert!(nodeinfo.channelcount > 0)
        } else {
            assert!(false, "No nodeinfo received");
        }
    }
}
