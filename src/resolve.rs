use futures::future::Future;
use futures::channel::oneshot;
use futures::channel::oneshot::*;
use std::sync::Mutex;
use std::collections::HashMap;
use std::pin::Pin;

type EndpointData = u64; // TODO

#[derive(Debug)]
pub struct Pending {
	id: u64,
	sender: Sender<Option<EndpointData>>,
}

trait ChannelResolving {
	fn get_endpoints(&self, id: u64) -> Pin<Box<dyn Future<Output = Result<Option<EndpointData>, Canceled>>>>;
	//async fn get_endpoints(&self, id: u64) -> Option<EndpointData>;
}
pub struct CachingChannelResolving {
	chan_id_cache: HashMap<u64, EndpointData>,
	pending: Mutex<Vec<Pending>>,
}

impl CachingChannelResolving {
	pub fn new() -> CachingChannelResolving {
		CachingChannelResolving{
			chan_id_cache: [(1u64, 1337u64), (2u64, 1338u64), (3u64, 1339u64)].into_iter().collect(),
			pending: Mutex::new(Vec::new()),
		}
	}

	pub fn resolve(&mut self) { // TODO
		let mut data = self.pending.lock().unwrap();
		
		while let Some(e) = data.pop() {
			if let Some(num) = self.chan_id_cache.get(&e.id) {
				e.sender.send(Some(*num));
			} else {
				e.sender.send(None);
			}
		}
	}
}

impl ChannelResolving for CachingChannelResolving {
	fn get_endpoints(&self, id: u64) -> Pin<Box<dyn Future<Output = Result<Option<EndpointData>, Canceled>>>> {
		//async fn get_endpoints(&self, id: u64) -> Option<EndpointData> {
		let (sender, receiver) = oneshot::channel::<Option<u64>>();

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