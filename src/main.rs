
use dvf::validation::{OperatorCommittee};
use dvf::validation::operator::TOperator;
use dvf::validation::operator::{RemoteOperator, LocalOperator};
use dvf::crypto::{ThresholdSignature};
use std::sync::Arc;
use types::Hash256;
use eth2_hashing::{Context, Sha256Context};
use hsconfig::Export as _;
use dvf::node::node::Node;
use hsconfig::{Committee, Secret};
use consensus::Committee as ConsensusCommittee;
use mempool::Committee as MempoolCommittee;
use std::fs;
use log::{info};
use futures::future::join_all;
use dvf::node::dvfcore::{DvfCore, DvfSigner};
use parking_lot::{RwLock};
use types::Keypair;
use tokio::time::{sleep, Duration};
use env_logger::Env;
use dvf::node::config::NodeConfig;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use futures::executor::block_on; 
use std::path::PathBuf;
use dvf::validation::operator_committee_definitions::OperatorCommitteeDefinition;


async fn start_nodes(n_nodes: usize) -> Result<Vec<Arc<Node>>, String> {
    let configs: Vec<NodeConfig> = (0..n_nodes).map(|i| {
        NodeConfig::default()
            .set_base_port((25_000 + i * 100) as u16)
            .set_base_dir(PathBuf::from(format!("./dvf_nodes/{}/", i)))
    }).collect();

    println!("Starting {} nodes", n_nodes);
    let mut nodes: Vec<Arc<Node>> = Vec::default();
    for i in 0..n_nodes {
        let node = Node::new(configs[i].clone()).await.expect("Should start node");
        nodes.push(Arc::new(node)); 
    }
    Ok(nodes)
}

async fn start_dvf_committee(validator_id: u64, nodes: &[Arc<Node>], threshold: usize) -> Result<(Keypair, Vec<DvfSigner>), String> {
    let mut m_threshold = ThresholdSignature::new(threshold);
    let (kp, kps, ids) = m_threshold.key_gen(nodes.len());

    let def = OperatorCommitteeDefinition {
        total: nodes.len() as u64,
        threshold: threshold as u64,
        validator_id,
        validator_public_key: kp.pk.clone(),
        operator_ids: ids.clone(),
        operator_public_keys: kps.iter().map(|x| x.pk.clone()).collect(),
        node_public_keys: nodes.iter().map(|x| x.secret.name.clone()).collect(),
        base_socket_addresses: nodes.iter().map(|x| x.config.base_address).collect(),
    };

    let committee_file = format!("committee_{}.json", validator_id);
    fs::remove_file(&committee_file);
    def.to_file(committee_file.clone()).map_err(|e| format!("Error saving committee def file. {:?}", e))?;

    //// Print the committee file.
    //let epoch = 1;
    //let mempool_committee = MempoolCommittee::new(
        //nodes
            //.iter()
            //.enumerate()
            //.map(|(i, node)| {
                //(node.secret.name, 
                //1, 
                //nodes[i].config.tx_receiver_address.clone(), 
                //nodes[i].config.mempool_receiver_address.clone(), 
                //nodes[i].config.dvfcore_receiver_address.clone(), 
                //nodes[i].config.signature_receiver_address.clone())
            //})
            //.collect(),
        //epoch,
    //);
    //let consensus_committee = ConsensusCommittee::new(
        //nodes
            //.iter()
            //.enumerate()
            //.map(|(i, node)| {
                //(node.secret.name, 
                //1,
                //nodes[i].config.consensus_receiver_address.clone())
            //})
            //.collect(),
        //epoch,
    //);
    //let _ = fs::remove_file(committee_file.as_str());
    //let committee = Committee {
        //mempool: mempool_committee,
        //consensus: consensus_committee,
    //};
  
    //committee.write(committee_file.as_str()).map_err(|e| e.to_string())?;

    let committee_def = OperatorCommitteeDefinition::from_file(committee_file)
        .map_err(|e| format!("Error in loading file. {:?}", e))?;
    println!("Starting DVF instances for validator {}", validator_id);
    let mut dvfs: Vec<DvfSigner> = Vec::default();
    for i in 0..nodes.len() {
        // create dvf consensus
        let node = nodes[i].clone();
        let operator_id = ids[i];
        let bls_kp = kps[i].clone();
        //let committee = Committee::read(committee_file.as_str()).unwrap();

        //let (tx_consensus, rx_consensus) = channel(100);

        //let mut operator_committee = OperatorCommittee::new(validator_id, kp.pk.clone(), threshold, rx_consensus);
        //let local_operator = Arc::new(
            //RwLock::new(LocalOperator::new(ids[i], Arc::new(kps[i].clone())))); 
        //operator_committee.add_operator(ids[i], local_operator);
        //for j in 0..nodes.len() {
            //if j == i {
                //continue
            //}
            //let signature_address = nodes[j].config.signature_receiver_address.clone();
            //let remote_operator = Arc::new(
                //RwLock::new(RemoteOperator::new(ids[j], kps[j].pk.clone(), signature_address)));  
            //operator_committee.add_operator(ids[j], remote_operator);
        //}

        let dvf_inst = DvfSigner::spawn(
            node,
            validator_id,
            operator_id,
            bls_kp,
            committee_def.clone(),
        ).await;
        dvfs.push(dvf_inst);
    }

    Ok((kp, dvfs))
}

async fn committee_sign(dvfs: &mut Vec<DvfSigner>, kp: Keypair, message: &str) -> Result<(), String> {
    println!("Start committee signing");

    let mut context = Context::new();
    context.update(message.as_bytes());
    let message_hash = Hash256::from_slice(&context.finalize());
    // Select the one that is supposed to propose a duty according to message hash
    let selected = message_hash.to_low_u64_le() as usize % dvfs.len();
    let dvf = &dvfs[selected];

    let sig1 = dvf.sign_str(message).await.map_err(|e| format!("failed to committee sign: {:?}", e))?;
    let sig2 = kp.sk.sign(message_hash);

    let status1 = sig1.verify(&kp.pk, message_hash);
    let status2 = sig2.verify(&kp.pk, message_hash);

    println!("Committee sign and verify: {}", status1);
    println!("Original sign and verify: {}", status2);
    Ok(())
}

async fn deploy_testbed(n_nodes: usize, threshold: usize) -> Result<(), String> {

    let mut logger = env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    logger.format_timestamp_millis();
    logger.init();

    let nodes = start_nodes(n_nodes).await?;
    
    let validator_id = 1;
    let (kp, mut dvfs) = start_dvf_committee(validator_id, &nodes, threshold).await?;
    
    committee_sign(&mut dvfs, kp, "hello world").await?;
    drop(dvfs);
    // Wait 10 seconds for background jobs to finish
    sleep(Duration::from_secs(10)).await;
    Ok(())
}


//#[tokio::main(worker_threads = 200)]
#[tokio::main]
async fn main() {
    let t: usize = 3;
    let n: usize = 5;
    match deploy_testbed(n, t).await {
        Ok(()) => println!("Testbed exited successfully"),
        Err(e) => {
            eprintln!("Testbed exited with error: {}", e);
            //std::process::exit(1)
        } 
    }

}
