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

use utils::helpers::save_bytes_to_file;

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

// ---- RLP helpers: compute total length of the next RLP item and peel a stream ----
fn rlp_item_total_len(input: &[u8]) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let b0 = input[0];

    match b0 {
        0x00..=0x7f => Some(1), // single byte
        0x80..=0xb7 => {
            let len = (b0 - 0x80) as usize;
            if input.len() < 1 + len { return None; }
            Some(1 + len)
        }
        0xb8..=0xbf => {
            let len_of_len = (b0 - 0xb7) as usize;
            if input.len() < 1 + len_of_len { return None; }
            let mut l: usize = 0;
            for &byte in &input[1..1 + len_of_len] {
                l = (l << 8) | (byte as usize);
            }
            if input.len() < 1 + len_of_len + l { return None; }
            Some(1 + len_of_len + l)
        }
        0xc0..=0xf7 => {
            let payload_len = (b0 - 0xc0) as usize;
            if input.len() < 1 + payload_len { return None; }
            Some(1 + payload_len)
        }
        0xf8..=0xff => {
            let len_of_len = (b0 - 0xf7) as usize;
            if input.len() < 1 + len_of_len { return None; }
            let mut l: usize = 0;
            for &byte in &input[1..1 + len_of_len] {
                l = (l << 8) | (byte as usize);
            }
            if input.len() < 1 + len_of_len + l { return None; }
            Some(1 + len_of_len + l)
        }
    }
}

// Peel the next RLP item as a byte-string, returning (payload_bytes, total_consumed)
// Returns None if the input is malformed or the item is an RLP list (not a string/bytes).
fn rlp_peel_string(input: &[u8]) -> Option<(Vec<u8>, usize)> {
    if input.is_empty() { return None; }
    let b0 = input[0];
    match b0 {
        0x00..=0x7f => {
            // single byte, payload is that byte
            Some((vec![b0], 1))
        }
        0x80..=0xb7 => {
            let len = (b0 - 0x80) as usize;
            if input.len() < 1 + len { return None; }
            let payload = input[1..1+len].to_vec();
            Some((payload, 1 + len))
        }
        0xb8..=0xbf => {
            let len_of_len = (b0 - 0xb7) as usize;
            if input.len() < 1 + len_of_len { return None; }
            let mut l: usize = 0;
            for &byte in &input[1..1 + len_of_len] {
                l = (l << 8) | (byte as usize);
            }
            if input.len() < 1 + len_of_len + l { return None; }
            let start = 1 + len_of_len;
            let payload = input[start..start + l].to_vec();
            Some((payload, 1 + len_of_len + l))
        }
        0xc0..=0xff => {
            // It's a list; Nitro segments are expected to be byte-strings.
            None
        }
    }
}

// Try to decode a concatenated stream of RLP items, returning each item's byte-string payload.
// If any item is not a byte-string or parsing fails, return None.
fn try_decode_rlp_segments(input: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut cursor = input;
    while !cursor.is_empty() {
        let (payload, consumed) = match rlp_peel_string(cursor) {
            Some(v) => v,
            None => return None,
        };
        out.push(payload);
        cursor = &cursor[consumed..];
    }
    if out.is_empty() { None } else { Some(out) }
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

fn decode_nitro_batch(batch: &[u8]) -> eyre::Result<()> {
    if let Some(segments) = try_decode_rlp_segments(batch) {
        println!("Decoded {} segments from Nitro batch", segments.len());
        handle_segments(segments)?;
        Ok(())
    } else {
        eyre::bail!("Failed to decode RLP segments from Nitro batch");
    }
}

fn handle_segments(segments: Vec<Vec<u8>>) -> eyre::Result<()> {
    for (i, seg) in segments.iter().enumerate() {
        if seg.is_empty() {
            println!("Segment {} is empty, skipping", i);
            continue;
        }
        let kind = seg[0];
        let payload = &seg[1..];

        match kind {
            0 => {
                // L2 message
                decode_l2_message(payload)?;
            }
            1 => {
                // L2 message, brotli-compressed
                if let Some(decompressed) = try_brotli_decompress(payload) {
                    decode_l2_message(&decompressed)?;
                } else {
                    println!("Segment {}: failed to brotli-decompress L2 message", i);
                }
            }
            2 => {
                // delayed messages pointer; optional to resolve
                println!("Segment {}: delayed messages pointer (ignored for now)", i);
            }
            other => {
                println!("Segment {}: unknown kind {}, raw {}", i, other, hex::encode(seg));
            }
        }
    }
    Ok(())
}

fn decode_l2_message(msg: &[u8]) -> eyre::Result<()> {
    if msg.is_empty() {
        eyre::bail!("Empty L2 message");
    }
    let kind = msg[0];
    let payload = &msg[1..];

    match kind {
        0x04 => {
            println!("L2 message kind=SignedTx (0x04), len={}", payload.len());
            // Do not force EIP-1559; try a general decoder.
            // Option A: Attempt alloy's general envelope decoding (legacy/2930/1559).
            // If not readily available, start by logging and saving the tx for offline decode.
            // Example placeholder:
            // match decode_eth_tx_envelope(payload) { ... }

            // For now, just log the first bytes and optionally persist:
            println!("SignedTx first 64: {}", hex::encode(&payload[..64.min(payload.len())]));
            // save_raw_bytes_to_file("l2_signed_tx.bin", payload.to_vec())?;
        }
        0x03 => {
            println!("L2 message kind=Batch (0x03), nested frames");
            let mut cursor = payload;
            let mut idx = 0usize;
            while cursor.len() >= 8 {
                let len = u64::from_be_bytes(cursor[0..8].try_into().unwrap()) as usize;
                cursor = &cursor[8..];
                if cursor.len() < len {
                    eyre::bail!("Frame length exceeds remaining buffer");
                }
                let frame = &cursor[..len];
                cursor = &cursor[len..];

                println!("Nested frame {}: {} bytes", idx, len);
                decode_l2_message(frame)?;
                idx += 1;
            }
            if !cursor.is_empty() {
                println!("Trailing {} bytes after frames (ignored)", cursor.len());
            }
        }
        0x09 => {
            println!("L2 message kind=synthetic delayed (0x09); ignoring");
        }
        other => {
            println!("Unknown L2 message kind=0x{:02x}, payload first 32 {}", other, hex::encode(&payload[..32.min(payload.len())]));
        }
    }
    Ok(())
}

fn handle_raw_blob(raw_blob: &[u8]) -> Result<()> {
    println!("raw_blob length: {}", raw_blob.len());
    println!("raw_blob first 64 bytes: {}", hex::encode(&raw_blob[..64.min(raw_blob.len())]));
    
    if let Some(&first) = raw_blob.first() {
        if (first & 0x80) != 0 {
            println!("Detected DAS/Anytrust header (0x80) flag.");

            if raw_blob.len() < 65 {
                eyre::bail!("raw_blob too short for DAS/Anytrust header");
            }
            let _keyset_hash = &raw_blob[1..33]; // Not needed to decode the payload
            let data_hash = &raw_blob[33..65];

            // Fetch from DAC using data_hash, then base64-encode 
            //decode_nitro_batch(&payload);
            println!("DAC not implemented yet!");
            return Ok(());
        } else if first == 0x00 {
            println!("Detected Nitro brotli header (0x00). Decompressing the body");
            let compressed = &raw_blob[1..];
            if let Some(decompressed) = try_brotli_decompress(compressed) {
                println!("Decompressed Brotli payload length: {}", decompressed.len());
                decode_nitro_batch(&decompressed)?;
                return Ok(());
            } else {
                eyre::bail!("Brotli decompression failed");
            }
        } else {
            // Unknown at top; probe in order
            println!("No explicit Nitro header; Probing RLP and Brotli Paths...");

            if let Some(segments) = try_decode_rlp_segments(raw_blob) {
                println!("RLP Segment stream detected (raw). Segment count: {}", segments.len());
                handle_segments(segments)?;
                return Ok(());
            }

            if let Some(decompressed) = try_brotli_decompress(raw_blob) {
                println!("Brotli compressed payload detected (raw). Decompressing...");
                if let Some(segments) = try_decode_rlp_segments(&decompressed) {
                    println!("RLP Segment stream detected (raw). Segment count: {}", segments.len());
                    handle_segments(segments)?;
                    return Ok(());
                }
            }

            if first == 0x03 || first == 0x04 || first == 0x09 {
                println!("Likely at L2 message boundary; parsing L2 message directly...");
                decode_l2_message(raw_blob)?;
                return Ok(());
            }

            eyre::bail!("Could not classify blob by header/probes. Share first 128 bytes for more triage.");
        }
    } else {
        eyre::bail!("Empty blob");
    }
    
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
                            // println!("data_storage_ref: {:#?}", &data_storage_ref);
                            let url = data_storage_ref.get(0).unwrap().get("url").unwrap().as_str().unwrap().to_string();
                            // println!("url: {}", &url);
                            let response = reqwest::get(&url).await?;
                            let raw_blob: Vec<u8> = response.bytes().await?.to_vec();
                            // println!("blob_data: {:#?}", &blob_data[..64]);

                            // println!("First bytes of raw blob : {}", hex::encode(&raw_blob));
                            save_bytes_to_file("raw_blob_1.txt", raw_blob.clone());
                            println!("trying to decompress the data using brotli");
                            
                            // The 4844 blob is usually raw; we keep the helper in case a provider returns
                            // a compressed blob directly.
                            // let raw_blob = match try_brotli_decompress(&raw_blob) {
                            //     Some(decompressed) => {
                            //         println!("Blob was Brotli compressed, decompressed to {} bytes", decompressed.len());
                            //         decompressed
                            //     }
                            //     None => {
                            //         println!("Blob is raw (not Brotli). First 64 bytes: {}", hex::encode(&raw_blob[..64.min(raw_blob.len())]));
                            //         raw_blob
                            //     }
                            // };
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
                            // ---- Decode end-to-end according to Nitro/DAS flow ----
                            if let Err(e) = handle_raw_blob(&raw_blob) {
                                eprintln!("handle_raw_blob failed: {e:?}");
                                continue;
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