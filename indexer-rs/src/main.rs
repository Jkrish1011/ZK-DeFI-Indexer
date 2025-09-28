use alloy::{
    contract::{ContractInstance, Interface},
    dyn_abi::DynSolValue,
    network::{EthereumWallet, TransactionBuilder, NetworkWallet},
    providers::{Provider, ProviderBuilder, WsConnect},
    primitives::{address, U256, hex, B256,Log as ETHLog, LogData, FixedBytes, Address},
    rpc::types::{Filter, Log, TransactionRequest, BlockNumberOrTag},
    signers::local::LocalSigner,
    sol
};

use std::str::from_utf8;
use alloy::sol_types::SolEvent;
use tokio::task::JoinHandle;
use std::panic;
use std::future::Future;
use chrono::format::Fixed;
use hex as justHex;
use std::fs::read_to_string;
use rand::thread_rng;
use std::path::Path;
use eyre::Result;
use futures_util::StreamExt;
use dotenv::dotenv;
use std::env;
use std::str::FromStr;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time};
use serde::{Deserialize, Serialize};
use reqwest::{Error, Client};
use brotli::Decompressor;
use std::io::Read;
use c_kzg::KzgSettings;
use c_kzg::{Blob, KzgCommitment};


sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    ARBITRUM,
    "src/abi/ARBITRUM.json"
}

sol! {
    #[derive(Debug)]
    struct TimeBounds {
        uint64 minTimestamp;
        uint64 maxTimestamp;
        uint64 minBlockNumber;
        uint64 maxBlockNumber;
    }

    #[derive(Debug)]
    enum BatchDataLocation {
        /// @notice The data can be found in the transaction call data
        TxInput,
        /// @notice The data can be found in an event emitted during the transaction
        SeparateBatchEvent,
        /// @notice This batch contains no data
        NoData,
        /// @notice The data can be found in the 4844 data blobs on this transaction
        Blob
    }

    #[derive(Debug)]
    event SequencerBatchDelivered(
        uint256 indexed batchSequenceNumber,
        bytes32 indexed beforeAcc,
        bytes32 indexed afterAcc,
        bytes32 delayedAcc,
        uint256 afterDelayedMessagesRead,
        TimeBounds timeBounds,
        BatchDataLocation dataLocation
    );
}

fn calculate_slot_number(blockNumber: u64) -> u64 {
    // const slot = Math.floor((timestamp - 1663224000) / 12) + 4700013;
    let slot = (blockNumber - 15537394) + 4700013;
    println!("slot: {}", &slot.to_string());
    slot
}

fn commitment_to_hex(commitment: &KzgCommitment) -> String {
    format!("0x{}", justHex::encode(commitment.as_ref()))
}

fn compute_kzg_commitment(blob: &[u8]) -> Option<KzgCommitment> {
    println!("Attempting KZG commitment computation on blob of size: {}", blob.len());
    let setup_path = "trusted_setup.txt";

    if !Path::new(setup_path).exists() {
        println!("ERROR: trusted_setup.txt doesn't exists!");
        return None;
    }

    let kzg_settings = match KzgSettings::load_trusted_setup_file(&Path::new(setup_path), 0) {
        Ok(settings) => settings,
        Err(e) => {
            println!("ERROR:Failed to load trusted setup: {:?}", e);
            return None;
        }
    };

    // Prepare blob data - KZG expects exactly 131072 bytes (4096 field elements * 32 bytes each)
    let required_size = 4096 * 32; // 131072 bytes
    let blob_data = if blob.len() == required_size {
        println!("Blob is already the correct size: {} bytes", required_size);
        blob.to_vec()
    } else if blob.len() < required_size {
        println!("Blob is too small ({}), padding to {} bytes", blob.len(), required_size);
        let mut padded = blob.to_vec();
        padded.resize(required_size, 0);
        padded
    } else {
        println!("Blob is too large ({}), truncating to {} bytes", blob.len(), required_size);
        blob[..required_size].to_vec()
    };

    let blob_converted = match Blob::from_bytes(&blob_data) {
        Ok(blob) => blob,
        Err(e) => {
            println!("ERROR:Failed to convert blob to KZG blob: {:?}", e);
            return None;
        }
    };

    let commitment: KzgCommitment = match kzg_settings.blob_to_kzg_commitment(&blob_converted) {
        Ok(commitment) => {
            let commitment_hex = commitment_to_hex(&commitment);
            println!("commitment: {}", &commitment_hex);
            commitment
        },
        Err(e) => {
            println!("ERROR:Failed to compute KZG commitment: {:?}", e);
            return None;
        }
    };
    println!("KZG commitment computed successfully");
    Some(commitment)
}

fn try_brotli_decompress(blob_data: &[u8]) -> Option<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut reader = Decompressor::new(blob_data, 4096);
    match reader.read_to_end(&mut decompressed) {
        Ok(_) => Some(decompressed),
        Err(e) => None
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let alchemy_url = env::var("ETHEREUM_MAINNET_WSS_URL")
        .expect("ETHEREUM_MAINNET_WSS_URL must be set in .env");
    let arbitrum_contract = env::var("ARBITRUM_CONTRACT_ADDRESS")
        .expect("ARBITRUM_CONTRACT_ADDRESS must be set in .env");
    let arbitrum_sequencer_inbox_contract = env::var("ARBITRUM_SEQUENCER_INBOX_CONTRACT")
        .expect("ARBITRUM_SEQUENCER_INBOX_CONTRACT must be set in .env");

    let blobscan_api = env::var("BLOBSCAN_API")
        .expect("BLOBSCAN_API must be set in .env");

    // Create WebSocket connection
    let ws = WsConnect::new(&alchemy_url);
    
    // Create provider with WebSocket transport
    let provider = ProviderBuilder::new()
        .connect_ws(ws)
        .await?;
    
    println!("Connected! Subscribing to new blocks...");

    // Parse the string (expects a 0x-prefixed hex address)
    let arbitrum_address: Address = arbitrum_contract
        .parse()
        .expect("ARBITRUM_CONTRACT_ADDRESS must be a valid 0x-prefixed address");

    let arbitrum_sequencer_inbox_address: Address = arbitrum_sequencer_inbox_contract
        .parse()
        .expect("ARBITRUM_SEQUENCER_INBOX_CONTRACT must be a valid 0x-prefixed address");

    let filter = Filter::new()
        // By NOT specifying an `event` or `event_signature` we listen to ALL events of the
        // contract.
        .address(arbitrum_sequencer_inbox_address)
        .from_block(BlockNumberOrTag::Latest);

    // Subscribe to logs.
    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        // Only attempt to decode if topic0 matches the SequencerBatchDelivered signature.
        if let Some(topic0) = log.topics().get(0) {
            if topic0 == &SequencerBatchDelivered::SIGNATURE_HASH {
                match SequencerBatchDelivered::decode_log(&log.inner) {
                    Ok(event) => {
                        println!("Received SequencerBatchDelivered event: {:#?}", &event);
                        println!("event.timeBounds.minBlockNumber: {}", event.timeBounds.minBlockNumber);
                        
                        let blobscanner_api = blobscan_api.clone().replace("BLOCK", event.timeBounds.minBlockNumber.to_string().as_str());
                        let response = reqwest::get(&blobscanner_api).await?;
                        let blob_data: serde_json::Value = response.json().await?;
                        
                        let blobs = blob_data.get("blobs").unwrap();

                        for blob in blobs.as_array().unwrap() {
                            let data_storage_ref = blob.get("dataStorageReferences").unwrap();
                            println!("data_storage_ref: {:#?}", &data_storage_ref);
                            let url = data_storage_ref.get(0).unwrap().get("url").unwrap().as_str().unwrap().to_string();
                            println!("url: {}", &url);
                            let response = reqwest::get(&url).await?;
                            let blob_data: Vec<u8> = response.bytes().await?.to_vec();
                            println!("blob_data: {:#?}", &blob_data[..64]);

                            println!("trying to decompress the data using brotli");

                            match try_brotli_decompress(&blob_data) {
                                Some(decompressed) => {
                                    println!("Blob is Brotli compressed! Decompressed size: {}", decompressed.len());
                                    println!("First 64 bytes (hex): {}", hex::encode(&decompressed[..64.min(decompressed.len())]));
                                }
                                None => {
                                    println!("Blob is raw (not Brotli). First 64 bytes: {}", hex::encode(&blob_data[..64.min(blob_data.len())]));
                                }
                            }

                            match compute_kzg_commitment(&blob_data) {
                                Some(commitment) => {
                                    println!("commitment: {:#?}", &commitment);
                                    println!("Commitment from event: {}", blob.get("commitment").unwrap().as_str().unwrap());
                                }
                                None => {
                                    println!("Failed to compute KZG commitment");
                                }
                            }
                        }

                    }
                    Err(e) => {
                        // This can still fail if the ABI or indexing expectations differ.
                        println!("Failed to decode SequencerBatchDelivered event: {:#?}", e);
                    }
                }
            } else {
                // event not in consideration
                // println!("Skipped non-target event with topic0: {:#?}", topic0);
            } 
        } else {
            println!("Received log without topics: {:#?}", log);
        }
    }

    Ok(())
}