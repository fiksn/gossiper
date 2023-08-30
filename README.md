# gossiper
Lightning gossip ingestion

This is yet another [LDK](https://lightningdevkit.org/) [demo](https://github.com/lightningdevkit/ldk-sample) and a way for me to learn [Rust](https://www.rust-lang.org/).

## Goal

The current goal of this tool is to determine whether a lightning node has connectivity issues just by listening to gossip messages. 

Trick is that peers disable their channel with a node as soon as TCP connection breaks. So if more than n (configurable threshold) nodes disable a channel with a specific node we can tell with a high probability that it is unavailable.
All without sending a single probe packet to the target.

The tool is completely stateless, everything it needs is queried through `QueryShortChannelIds` and cached later on.

## Fingerprinting lightning implementations

While developing this I have found a simple way to fingerprint lightning node implementations.

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

## Example

```
2023-08-30 17:16:52.156 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 28 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 836085134498594816 direction: 1 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 28 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 819663928352047104 direction: 1 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 820341227486904321 direction: 1 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 28 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 882218443410571264 direction: 1 node: 03fbc17549ec667bccf397ababbcb4cdc0e3394345e4773079ab2774612ec9be61 alias: node201.fmt.mempool.space

2023-08-30 17:16:52.157 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 28 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 849020888796364801 direction: 0 node: 02ad4afb6e50ae4635ec5ddf5a57c44d4cc4b376ac6580f78cda0454a86e5fa6c2 alias: wyssblitz

2023-08-30 17:16:52.157 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 29 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 857100100212817921 direction: 0 node: 021e3544a3b10379e833b85600f05de6d162a8ff93945eebd7def493fc024f1962 alias: TheFlash⚡

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 829880590372896769 direction: 0 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 819762884369842176 direction: 1 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 29 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:100] ENABLE chid: 749630535234158592 direction: 1 node: 03c445275ee7d79ee5778ca2ac81b7c4d84aed7ee04629e8d8f35434b3c21e2da8 alias: Obi-Wan Cryptobi

2023-08-30 17:16:52.157 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 30 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom

2023-08-30 17:16:52.157 TRACE [gossiper::voter:50] DISABLE chid: 820341227486904321 direction: 0 node: 025f1456582e70c4c06b61d5c8ed3ce229e6d0db538be337a2dc6d163b0ebc05a5 alias: Moon (paywithmoon.com)

2023-08-30 17:16:52.157 INFO  [gossiper::voter:73] THRESHOLD BREACHED num: 30 node: 036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548 alias: Dawn of Freedom
```

Currently the output is very crude but you can detect that `036d306e01a65128e5130c99a004119f36bc296ee6f002a7c3c478530b8249a548` actually seemed to have some issues during testing.
TODO: Need to fix the false positives that come from bigger nodes where a lot of people simultaneously have connectivity issues independently. Since this is stateless it is hard to tell number of channels for a node (unless I do a complete sync but that is slow).
