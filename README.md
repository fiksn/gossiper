# gossiper
Lightning gossip ingestion

This is yet another [LDK](https://lightningdevkit.org/) [demo](https://github.com/lightningdevkit/ldk-sample) and a way for me to learn [Rust](https://www.rust-lang.org/).

## Goal

The current goal of this tool is to determine whether a lightning node has connectivity issues just by listening to gossip messages. 

Trick is that peers disable their channel with a node as soon as TCP connection breaks. So if more than n (configurable threshold) nodes disable a channel with a specific node we can tell with a high probability that the given lightning node is unavailable. 

All without sending a single probe packet to the target.

The tool is completely stateless, everything it needs is queried through `QueryShortChannelIds` and cached later one.

## Fingerprinting Lightning implementations

While developing this I have found a simple way to fingerprint lightnig node implementations.

* LND internally creates a `GossipSyncer` for the connection only when node advertises it understands `gossip_queries` [BOLT-09](https://github.com/lightning/bolts/blob/master/09-features.md)
  So if you don't do that `QueryShortChannelIds` will not yield any reply. All other implementations do not need this feature advertised.
* Eclair: will return data also when `query_short_channel_ids` [BOLT-07](https://github.com/lightning/bolts/blob/master/07-routing-gossip.md) `chain_hash` is invalid.

You can send something like
```
&msgs::QueryShortChannelIds {
  chain_hash: BlockHash::from_hex("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce27f"),
  short_channel_ids: vec![869059488412139521],
}
``` 
(bitcoin genesis hash is `000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f` btw)

and stil get a normal reply.

* CLN: does not do any of the previous things (which is unique too)
