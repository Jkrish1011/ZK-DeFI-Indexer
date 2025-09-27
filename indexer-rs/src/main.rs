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
use std::path::PathBuf;
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