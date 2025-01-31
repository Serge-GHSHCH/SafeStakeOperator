use crate::config::Committee;
use crate::consensus::{ConsensusMessage, CHANNEL_CAPACITY};
use crate::error::ConsensusResult;
use crate::messages::{Block, QC};
use bytes::Bytes;
use crypto::Hash as _;
use crypto::{Digest, PublicKey};
use futures::stream::futures_unordered::FuturesUnordered;
use futures::stream::StreamExt as _;
use log::{debug, error, info, warn};
use network::{SimpleSender, DvfMessage, VERSION};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use store::Store;
use tokio::sync::mpsc::{Receiver};
use tokio::time::{sleep, Duration, Instant};
use std::net::SocketAddr;
use tokio::time::timeout;
use crate::error::ConsensusError;
use utils::monitored_channel::{MonitoredChannel, MonitoredSender};

#[cfg(test)]
#[path = "tests/synchronizer_tests.rs"]
pub mod synchronizer_tests;

const TIMER_ACCURACY: u64 = 5_000;

pub struct Synchronizer {
    store: Store,
    inner_channel: MonitoredSender<Block>,
}

impl Synchronizer {
    pub fn new(
        name: PublicKey,
        committee: Committee,
        store: Store,
        tx_loopback: MonitoredSender<Block>,
        sync_retry_delay: u64,
        validator_id: u64,
        exit: exit_future::Exit
    ) -> Self {
        let mut network = SimpleSender::new();
        let (tx_inner, mut rx_inner): (_, Receiver<Block>) = MonitoredChannel::new(CHANNEL_CAPACITY, "sync-inner".to_string(), "info");

        let store_copy = store.clone();
        tokio::spawn(async move {
            let mut waiting = FuturesUnordered::new();
            let mut pending = HashSet::new();
            let mut requests = HashMap::new();

            let timer = sleep(Duration::from_millis(TIMER_ACCURACY));
            tokio::pin!(timer);
            loop {
                let exit = exit.clone();
                tokio::select! {
                    Some(block) = rx_inner.recv() => {
                        if pending.insert(block.digest()) {
                            let parent = block.parent().clone();
                            let author = block.author;
                            let fut = Self::waiter(store_copy.clone(), parent.clone(), block);
                            waiting.push(fut);

                            if !requests.contains_key(&parent) {
                                debug!("Requesting sync for block {}", parent);
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .expect("Failed to measure time")
                                    .as_millis();
                                requests.insert(parent.clone(), now);
                                let address = committee
                                    .address(&author)
                                    .expect("Author of valid block is not in the committee");
                                let message = ConsensusMessage::SyncRequest(parent, name);
                                let message = bincode::serialize(&message)
                                    .expect("Failed to serialize sync request");
                                let dvf_message = DvfMessage { version: VERSION, validator_id: validator_id, message: message};
                                let serialized_msg = bincode::serialize(&dvf_message).unwrap();
                                debug!("[SYNC] Sending to {:?}", address);
                                network.feed(address, Bytes::from(serialized_msg)).await;
                            }
                        }
                    },
                    Some(result) = waiting.next() => match result {
                        Ok(block) => {
                            let _ = pending.remove(&block.digest());
                            let _ = requests.remove(block.parent());
                            if let Err(e) = tx_loopback.send(block).await {
                                panic!("Failed to send message through core channel: {}", e);
                            }
                        },
                        Err(ConsensusError::StoreReadTimeout {wait_on, deliver}) => {
                            warn!("Failed to retrieve parent {} for block {}", wait_on, deliver);
                            let _ = pending.remove(&deliver.digest());
                            let _ = requests.remove(&wait_on);
                        },
                        Err(e) => error!("{}", e)
                    },
                    () = &mut timer => {
                        let mut i: u64 = 0;
                        debug!("[VA {}] Sync timer with {} requests", validator_id, requests.len());
                        let addresses: Vec<SocketAddr> = committee
                            .broadcast_addresses(&name)
                            .into_iter()
                            .map(|(_, x)| x)
                            .collect();

                        match timeout(Duration::from_millis(TIMER_ACCURACY), network.broadcast_flush(addresses.clone())).await {
                            Ok(_) => {
                                // This implements the 'perfect point to point link' abstraction.
                                for (digest, timestamp) in requests.iter_mut() {
                                    let now = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .expect("Failed to measure time")
                                        .as_millis();
                                    if *timestamp + (sync_retry_delay as u128) < now && !pending.contains(&digest) {
                                        debug!("Requesting sync for block {} (retry)", digest);
                                        let message = ConsensusMessage::SyncRequest(digest.clone(), name);
                                        let message = bincode::serialize(&message)
                                            .expect("Failed to serialize sync request");
                                        let dvf_message = DvfMessage { version: VERSION, validator_id: validator_id, message: message};
                                        let serialized_msg = bincode::serialize(&dvf_message).unwrap();
                                        debug!("[SYNC] Broacasting to {:?}", addresses);
                                        network.broadcast_feed(addresses.clone(), Bytes::from(serialized_msg)).await;
                                        // network.lucky_broadcast_feed(addresses.clone(), Bytes::from(serialized_msg), 1).await;
                                        info!("[VA {}] Sync broadcast {} : {}.", validator_id, i, digest);
                                        *timestamp = now;
                                    }
                                    i = i+1;
                                }
                                network.broadcast_flush(addresses).await;
                            },
                            Err(_) => {
                                warn!("Network is busy. Delay syncing requests...")
                            }
                        }
                        timer.as_mut().reset(Instant::now() + Duration::from_millis(TIMER_ACCURACY));
                    },
                    () = exit => {
                        break;
                    }
                }
            }
        });
        Self {
            store,
            inner_channel: tx_inner,
        }
    }

    // async fn waiter(store: Store, wait_on: Digest, deliver: Block) -> ConsensusResult<Block> {
    //     let _ = store.notify_read(wait_on.to_vec()).await?;
    //     Ok(deliver)
    // }

    async fn waiter(store: Store, wait_on: Digest, deliver: Block) -> ConsensusResult<Block> {
        match timeout(Duration::from_secs(20), store.notify_read(wait_on.to_vec())).await {
            Ok(read_result) => {
                let _ = read_result?;
                Ok(deliver)
            }
            Err(_) => {
                Err(ConsensusError::StoreReadTimeout {wait_on, deliver})
            }
        }
    }

    pub async fn get_parent_block(&mut self, block: &Block) -> ConsensusResult<Option<Block>> {
        if block.qc == QC::genesis() {
            return Ok(Some(Block::genesis()));
        }
        let parent = block.parent();
        match self.store.read(parent.to_vec()).await? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => {
                if let Err(e) = self.inner_channel.send(block.clone()).await {
                    panic!("Failed to send request to synchronizer: {}", e);
                }
                Ok(None)
            }
        }
    }

    // pub async fn get_ancestors(
    //     &mut self,
    //     block: &Block,
    // ) -> ConsensusResult<Option<(Block, Block)>> {
    //     let b1 = match self.get_parent_block(block).await? {
    //         Some(b) => b,
    //         None => return Ok(None),
    //     };
    //     let b0 = self
    //         .get_parent_block(&b1)
    //         .await?
    //         .expect("We should have all ancestors of delivered blocks");
    //     Ok(Some((b0, b1)))
    // }

    pub async fn get_ancestors(
        &mut self,
        block: &Block,
    ) -> ConsensusResult<Option<(Block, Block)>> {
        let b1 = match self.get_parent_block(block).await? {
            Some(b) => b,
            None => return Ok(None)
        };
        let b0 = match self.get_parent_block(&b1).await? {
            Some(b) => b,
            None => return Ok(None),
        };
        Ok(Some((b0, b1)))
    }
}
