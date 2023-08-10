use futures::future::Future;
use futures::channel::oneshot;
use futures::channel::oneshot::*;
use std::sync::Mutex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::pin::Pin;
//use super::DummyPeerManager;

type EndpointData = u64; // TODO

#[derive(Debug)]
pub struct Pending {
	id: u64,
	sender: Sender<EndpointData>,
}

trait ChannelResolving {
	fn get_endpoints(&self, id: u64) -> Pin<Box<dyn Future<Output = Result<EndpointData, Canceled>>>>;
}
pub struct CachingChannelResolving {
	chan_id_cache: Mutex<HashMap<u64, EndpointData>>,
	pending: Mutex<Vec<Pending>>,
	//peer_manager: Option<DummyPeerManager>,
}

impl CachingChannelResolving {
	pub fn new() -> CachingChannelResolving {
		CachingChannelResolving{
			chan_id_cache: Mutex::new([(1u64, 1337u64), (2u64, 1338u64), (3u64, 1339u64)].into_iter().collect()),
			pending: Mutex::new(Vec::new()),
			//peer_manager: peer_manager,
		}
	}

	pub fn get_todo(&self) -> HashSet::<u64> {
		let mut set = HashSet::<u64>::new();

		let data = self.pending.lock().unwrap();
		for one in &*data {
			if self.chan_id_cache.lock().unwrap().get(&one.id) == None {
				set.insert(one.id);
			}
		}
		
		//self.peer_manager.unwrap()

		set
	}

	// if todo empty call resolve else call todo and call resolve after result arrives
	pub fn resolve(&mut self) { // TODO
		let mut data = self.pending.lock().unwrap();
		
		while let Some(e) = data.pop() {
			if let Some(num) = self.chan_id_cache.lock().unwrap().get(&e.id) {
				e.sender.send(*num);
			}
		}
	}
}

impl ChannelResolving for CachingChannelResolving {
	fn get_endpoints(&self, id: u64) -> Pin<Box<dyn Future<Output = Result<EndpointData, Canceled>>>> {
		let (sender, receiver) = oneshot::channel::<EndpointData>();

		self.pending.lock().unwrap().push(Pending { id: id, sender: sender});
		
		Box::pin(receiver)
	}
}
  
#[cfg(test)]
mod tests {
    use super::*;
	use tokio::test;

    //#[test]
	#[tokio::test]
    async fn test_resolve() {
		let mut resolver = CachingChannelResolving::new();

		let a = resolver.get_endpoints(1);
		resolver.resolve();
		
		let result = a.await;

		println!("{:?}", result);

		panic!("Foo");
    }
}