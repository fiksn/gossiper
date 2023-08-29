use std::sync::{Arc, Mutex};
use crate::CachingChannelResolving;
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
}


