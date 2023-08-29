use std::sync::{Arc, Mutex};
use super::resolve::*;

pub struct Voter {
    resolver: Mutex<Option<Arc<CachingChannelResolving>>>,
}
impl Voter {
	pub fn new() -> Voter {
		Voter{
            resolver: Mutex::new(None),
        }
    }

    pub fn register_resolver(&self, resolver: Arc<CachingChannelResolving>) {
		*(self.resolver.lock().unwrap()) = Some(resolver.clone());
	}

    pub async fn burek(&self) {
        println!("BUREK");
        let res = self.resolver.lock().unwrap().clone().unwrap();
        //res.get_node(res.get_endpoints_async(chanid)).expect("channel data").nodes[direction]).unwrap();
        res.get_endpoints_async(123u64).await;
    }
}


