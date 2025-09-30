use alloy::{
    contract::{ContractInstance, Interface},
    dyn_abi::DynSolValue,
    network::{EthereumWallet, TransactionBuilder, NetworkWallet},
    providers::{Provider, ProviderBuilder, WsConnect},
    primitives::{address, U256, hex, B256,Log as ETHLog, LogData, FixedBytes, Address, Bytes},
    rpc::types::{Filter, Log, TransactionRequest, BlockNumberOrTag},
    signers::local::LocalSigner,
    sol,
    sol_types::SolEvent,
    rlp::{Decodable, Encodable},
    consensus::{
        EthereumTxEnvelope,
        TypedTransaction,
        Transaction,
        transaction::{TxEip4844Variant, SignerRecoverable},
    },
};

use std::{
    str::{from_utf8,FromStr},
    panic,
    future::Future,
    fs::read_to_string,
    path::Path,
    env,
    collections::HashSet, 
    sync::Arc, 
    time::Duration,
    io::Read,
};

use tokio::{
    task::JoinHandle,
    sync::RwLock, 
    time,
};

use c_kzg::{
    KzgSettings,
    Blob,
    KzgCommitment,
};

use chrono::format::Fixed;
use hex as justHex;
use rand::thread_rng;
use eyre::Result;
use futures_util::StreamExt;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use reqwest::{Error, Client};
use brotli::Decompressor;

mod utils;

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

fn compute_kzg_commitment(blob: &[u8]) -> Option<String> {
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

    let commitment: Option<String> = match kzg_settings.blob_to_kzg_commitment(&blob_converted) {
        Ok(commitment) => {
            let commitment_hex = commitment_to_hex(&commitment);
            // println!("commitment: {}", &commitment_hex);
            Some(commitment_hex)
        },
        Err(e) => {
            println!("ERROR:Failed to compute KZG commitment: {:?}", e);
            None
        }
    };
    println!("KZG commitment computed successfully");
    commitment
}

fn try_brotli_decompress(blob_data: &[u8]) -> Option<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut reader = Decompressor::new(blob_data, 4096);
    match reader.read_to_end(&mut decompressed) {
        Ok(_) => Some(decompressed),
        Err(e) => None
    }
}


// Decode a single raw tx into alloy's TxEnvelope
// Fixed: Using TxEip4844Variant as the generic parameter
fn decode_tx(raw: &[u8]) -> Result<EthereumTxEnvelope<TxEip4844Variant>> {

    if raw.first().copied() != Some(0x02) {
        eyre::bail!("decode_tx called with non EIP 4844 payload (first byte = {:#x?})", raw.first());
    }
    let mut slice = raw;
    let tx = EthereumTxEnvelope::decode(&mut slice)?;
    Ok(tx)
}

/// Parse the *raw‑batch* format used by Arbitrum sequencer inboxes.
///
/// Returns a vector where each element is the **raw bytes of a single transaction**.
pub fn decode_raw_batch(data: &[u8]) -> Result<Vec<Vec<u8>>> {
    // Strip the 4-byte version field (big-endian, we ignore it
    if data.len() < 4 {
        println!("payload too short for version field!");
        return Ok(Vec::new());
    }

    let _version = u32::from_be_bytes(data[0..4].try_into().unwrap());
    let mut cursor = &data[4..];

    // Read transaction count (little-endian u32)
    if cursor.len() < 4 {
        println!("payload too short for transaction count!");
        return Ok(Vec::new());
    }

    let tx_count = u32::from_le_bytes(cursor[0..4].try_into().unwrap()) as usize;
    println!("Raw-batch header reports {} transactions", tx_count);
    cursor = &cursor[4..];

    // Sanity‑check: the reported size should not exceed the remaining buffer.
    if tx_count > cursor.len() {
        println!("Warning: reported size ({}) > actual remaining ({}) will decode until buffer ends", tx_count, cursor.len());
    }

    // Repeatedly parse and decode RLP-encoded transaction.
    let mut txs = Vec::new();
    let mut remaining = cursor;

    while !remaining.is_empty() {
        // The first byte of a valid 4844 tx must be 0x02.
        // If it isn’t, we abort – this protects us from mis‑aligned data.
        if remaining[0] != 0x02 {
            eyre::bail!(
                "expected transaction type 0x02 but found {:#x} at offset {}",
                remaining[0],
                data.len() - remaining.len()
            );
        }

        // `decode_tx` will consume exactly one transaction and return the envelope.
        // We clone the slice first so we can keep the raw bytes for later use.
        let raw_tx = remaining.to_vec(); // full remaining buffer
        let envelope = decode_tx(&raw_tx)?;

        // Determine how many bytes were consumed by the envelope.
        // `EthereumTxEnvelope::decode` consumes from the slice we give it,
        // leaving the remainder behind. We can reuse that logic here:
        let mut slice = &raw_tx[..];
        let _ = EthereumTxEnvelope::<TxEip4844Variant>::decode(&mut slice)?;
        let consumed = raw_tx.len() - slice.len();

        // Store the exact raw transaction bytes.
        txs.push(raw_tx[..consumed].to_vec());

        // Advance the cursor.
        remaining = &remaining[consumed..];
    }

    // the number of extracted txs should match the header.
    if txs.len() != tx_count {
        println!("Warning: header claimed {} txs but we extracted {}", tx_count, txs.len());
    }
    Ok(txs)
}

// Extract Core Fields from a TxEnvelope
// Fixed: Using TxEip4844Variant as the generic parameter
fn extract_core_fields(tx: &EthereumTxEnvelope<TxEip4844Variant>) -> eyre::Result<()> {
    let from: Address = tx.recover_signer()?;
    
    // Analyze EIP-1559. Might vary.
    let to: Option<Address> = tx.to();
    let value: U256 = tx.value();
    let data: Bytes = tx.input().clone();
    let nonce: u64 = tx.nonce();

    println!("from: {from:?}");
    println!("to: {to:?}");
    println!("value: {value}");
    println!("nonce: {nonce}");
    println!("calldata (first 16 bytes): 0x{}", hex::encode(&data[..16.min(data.len())]));

    Ok(())
}

// Returns the bytes that contains the transaction list
/// * If the blob starts with the 4‑byte version + 4‑byte little‑endian size
///   (the standard 4844 header), the function slices out the payload
///   described by that size.
/// * If the blob is shorter than 8 bytes or the size field is bogus,
///   an error is returned.
/// * The returned `Vec<u8>` may still start with a Nitro flag (`0x0a`/`0x0b`);
///   the caller decides what to do with it.
fn extract_payload_from_4844_blob(blob: &[u8]) -> Result<Vec<u8>> {
    // if length is < 8, treat the whole as the paylod
    if blob.len() < 8 {
        return Ok(blob.to_vec());
    }

    //4 - byte version (big-endian) - not under consideration, just read it
    let _version = u32::from_be_bytes(blob[0..4].try_into().unwrap());

    //4 - byte size (little-endian) length of the following data
    let size = u32::from_le_bytes(blob[4..8].try_into().unwrap());

    // check malformed size fields
    if size == 0 || 8 + size > blob.len() as u32 {
        // Size field looks wrong - assume it's the whole blob
        return Ok(blob.to_vec());
    }

    return Ok(blob[8..8 + size as usize].to_vec());
}

/// Parse a Nitro batch payload.  
/// Works for:
///   * flagged batches (`0x0a` plain, `0x0b` compressed)  
///   * flag‑less batches (some blobs from Blobscan)
/*
    When the first byte is not 0x0a/0x0b the data you receive is not a Nitro‑batch‑encoded list of (len || tx).
    Arbitrum stores those batches in a different layout that looks like

    +-------------------+-------------------+-------------------+
    | 4‑byte version   | 4‑byte tx‑count   | tx₀ … txₙ          |
    +-------------------+-------------------+-------------------+
*/
fn parse_nitro_batch(payload: &[u8]) -> eyre::Result<Vec<Vec<u8>>> {
    println!("payload size: {}", payload.len());
    
    // Detects the optional flag byte (0x0a or 0x0b – the two Nitro batch types).
    // Strip the optional batch‑type flag (0x0a = normal, 0x0b = compressed)
    let (mut data, start_offset) =  match payload.first() {
        Some(0x0a) => {
            println!("Detected nitro Flag (plain batch)");
            (payload[1..].to_vec(), 0usize)
        }
        Some(0x0b) => {
            println!("Detected nitro Flag (compressed batch)");
            (payload[1..].to_vec(), 0usize)
        }
        _ => {
            println!("No nitro flag detected");
            (payload.to_vec(), 0usize)
        }
    };


    // If the flag was 0x0b, the inner data is Brotli‑compressed.

    if let Some(&0x0b) = payload.first() {
        if let Some(decompressed) = try_brotli_decompress(&data) {
            println!(
                "Decompressed Brotli payload: {} → {} bytes",
                data.len(),
                decompressed.len()
            );
            data = decompressed;
        } else {
            println!("Brotli decompression failed! treating as raw");
        }
    }
    

    // Reading through the remaining bytes, reading little‑endian lengths.
    let mut txs = Vec::new();
    let mut offset = start_offset;

    while offset + 4 <= data.len() {
        // Read 4-byte little-endian length
        let len_bytes = &data[offset..offset + 4];
        let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
        offset += 4;

        // Checking if corrupted data or something else is wrong
        if len == 0 {
            println!("Zero-length entry at offset {}, skipping", offset - 4);
            continue;
        }

        // 10MB is far larger so check if we're going to overflow
        if len > 10_000_000 {
            println!("Unreasonable length {} at offset {}, skipping", len, offset - 4);
            break;
        }

        // Check if overflow
        if offset + len > data.len() {
            println!("Length {} would overflow buffer at offset {}, remaining {} bytes, skipping", len, offset, data.len() - offset);
            break;
        }
        
        let tx_bytes = data[offset..offset + len].to_vec();
        println!("Extracted tx #{} - {} bytes", txs.len(), len);
        txs.push(tx_bytes);

        offset += len;
    }

    println!("Finished parsing – {} transaction(s) extracted", txs.len());
    Ok(txs)
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
                            // ---- fetch the raw 4844 blob ---------------
                            let data_storage_ref = blob.get("dataStorageReferences").unwrap();
                            println!("data_storage_ref: {:#?}", &data_storage_ref);
                            let url = data_storage_ref.get(0).unwrap().get("url").unwrap().as_str().unwrap().to_string();
                            println!("url: {}", &url);
                            let response = reqwest::get(&url).await?;
                            let raw_blob: Vec<u8> = response.bytes().await?.to_vec();
                            // println!("blob_data: {:#?}", &blob_data[..64]);

                            println!("trying to decompress the data using brotli");
                            
                            // The 4844 blob is usually raw; we keep the helper in case a provider returns
                            // a compressed blob directly.
                            let raw_blob = match try_brotli_decompress(&raw_blob) {
                                Some(decompressed) => {
                                    println!("Blob was Brotli compressed, decompressed to {} bytes", decompressed.len());
                                    decompressed
                                }
                                None => {
                                    println!("Blob is raw (not Brotli). First 64 bytes: {}", hex::encode(&raw_blob[..64.min(raw_blob.len())]));
                                    raw_blob
                                }
                            };
                            // ---- compute and compare KZG commitment ----
                            match compute_kzg_commitment(&raw_blob) {
                                Some(commitment) => {
                                    println!("commitment: {}", &commitment);
                                    println!("Commitment from event: {}", blob.get("commitment").unwrap().as_str().unwrap());
                                    if commitment != blob.get("commitment").unwrap().as_str().unwrap() {
                                        println!("Commitment does not match");
                                        continue;
                                    } else {
                                        println!("Commitment matches");
                                    }
                                }
                                None => {
                                    println!("Failed to compute KZG commitment");
                                    continue;
                                }
                            }
                            // ---- Extract the inner payload (Nitro batch or plain tx list) ----
                            let inner_payload = extract_payload_from_4844_blob(&raw_blob)?;
                            if let Some(&first) = inner_payload.first() {
                                println!("inner_payload first byte!: {}", hex::encode([first]));
                            }
                            // Choose parser based on the first byte
                            let frames = if inner_payload.first().map_or(false, |b| *b == 0x0a || *b == 0x0b) {
                                println!("Detected Nitro flag");
                                parse_nitro_batch(&inner_payload)?
                            } else {
                                println!("No Nitro flag detected treating as raw batch");
                                decode_raw_batch(&inner_payload)?
                            };

                            // Decode each transaction
                            for (i, raw_tx) in frames.iter().enumerate() {
                                match decode_tx(raw_tx) {
                                    Ok(envelope) => {
                                        if let Err(e) = extract_core_fields(&envelope) {
                                            eprintln!("Tx #{i} – core‑field extraction failed: {e:?}");
                                        }
                                    }
                                    Err(e) => eprintln!("Tx #{i} – decode failed: {e:?}"),
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