use clap::Parser;
use serde::Serialize;
use eyre::{Result, eyre, WrapErr};
use std::io::Read;
use hex as justHex;

// alloy for versioned hash computation and typed 48-byte arrays
use alloy::{
    consensus::Bytes48,
    eips::eip4844::kzg_to_versioned_hash,
    primitives::B256,
};

use crate::utils::constants::*;

fn to_0x_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    s.push_str("0x");
    for b in bytes {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

fn process_blob(blob: &[u8], preview_words: usize) -> (usize, usize, bool, Vec<String>) {
    // Split into 32-byte words
    let total_words = (blob.len() + 31) / 32;

    // Preview first N words
    let mut first_words = Vec::new();
    for i in 0..total_words.min(preview_words) {
        let start = i * 32;
        let end = ((i + 1) * 32).min(blob.len());
        first_words.push(to_0x_hex(&blob[start..end]));
    }

    // EIP-4844 blob size check: 4096 * 32 bytes
    let is_exact_eip4844_size = blob.len() == 4096 * 32;

    (blob.len(), total_words, is_exact_eip4844_size, first_words)
}

/// Optionally compute the versioned hash from a 48-byte KZG commitment (EIP-4844).
fn compute_versioned_hash_from_commitment(commitment: &[u8; 48]) -> String {
    let cmt = Bytes48::from_slice(commitment);
    let vh = kzg_to_versioned_hash(cmt.as_ref());
    format!("{vh:#x}")
}

/// Extracts payload bytes assuming the common 31-bytes-per-32-byte-field scheme.
/// - blob must be exactly 4096 * 32 = 131_072 bytes.
/// - Returns concatenated 31-byte chunks (length 126,976 bytes).
pub fn extract_payload_31_per_fe(blob: &[u8]) -> Option<Vec<u8>> {
    if blob.len() != 4096 * 32 {
        eprintln!("Blob is not 131072 bytes; cannot apply 31-per-FE extraction");
        return None;
    }
    let mut out = Vec::with_capacity(4096 * 31);
    for i in 0..4096 {
        let start = i * 32;
        out.extend_from_slice(&blob[start..start + 31]);
    }
    Some(out)
}

/// Heuristic: trim trailing zero bytes often used as padding.
pub fn trim_trailing_zeros(mut v: Vec<u8>) -> Vec<u8> {
    while v.last().copied() == Some(0) {
        v.pop();
    }
    v
}

/// Try brotli-decompress a buffer; return None if it fails.
pub fn try_brotli_decompress(data: &[u8]) -> Option<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut reader = brotli::Decompressor::new(data, 4096);
    std::io::copy(&mut reader, &mut decompressed).ok()?;
    Some(decompressed)
}

// ---------- Minimal RLP decoding (string items only) ----------
//
// We only need to iteratively decode a concatenated stream of RLP "string" items,
// returning each item’s decoded bytes (no list support). This matches Nitro segment encoding.

#[derive(Debug)]
enum RlpKind {
    SingleByte,       // < 0x80
    ShortString(u8),  // 0x80..=0xb7, len = prefix - 0x80
    LongString(usize, usize), // 0xb8..=0xbf, (len_of_len, len)
    ShortList(u8),    // Not expected for Nitro segments
    LongList(usize),  // Not expected for Nitro segments
}

fn rlp_peek(input: &[u8]) -> Result<(RlpKind, usize)> {
    if input.is_empty() {
        return Err(eyre!("RLP: empty input"));
    }
    let b0 = input[0];
    match b0 {
        0x00..=0x7f => Ok((RlpKind::SingleByte, 1)),
        0x80..=0xb7 => {
            let len = (b0 - 0x80) as usize;
            Ok((RlpKind::ShortString(b0 - 0x80), 1 + len))
        }
        0xb8..=0xbf => {
            let len_of_len = (b0 - 0xb7) as usize;
            if input.len() < 1 + len_of_len {
                return Err(eyre!("RLP long string: insufficient bytes for len_of_len"));
            }
            let mut l: usize = 0;
            for &c in &input[1..1 + len_of_len] {
                l = (l << 8) | (c as usize);
            }
            Ok((RlpKind::LongString(len_of_len, l), 1 + len_of_len + l))
        }
        0xc0..=0xf7 => Ok((RlpKind::ShortList(b0 - 0xc0), 0)), // not supported
        0xf8..=0xff => {
            let len_of_len = (b0 - 0xf7) as usize;
            Ok((RlpKind::LongList(len_of_len), 0)) // not supported
        }
    }
}

/// Decode one RLP "string" item, returning (decoded_bytes, total_consumed).
/// Returns error if the next item is a list (Nitro segments are expected to be strings).
fn rlp_decode_one_string(input: &[u8]) -> Result<(Vec<u8>, usize)> {
    let (kind, total_len) = rlp_peek(input)?;
    match kind {
        RlpKind::SingleByte => Ok((vec![input[0]], 1)),
        RlpKind::ShortString(len_b) => {
            let len = len_b as usize;
            if input.len() < 1 + len {
                return Err(eyre!("RLP short string: insufficient bytes"));
            }
            Ok((input[1..1 + len].to_vec(), 1 + len))
        }
        RlpKind::LongString(len_of_len, len) => {
            if input.len() < 1 + len_of_len + len {
                return Err(eyre!("RLP long string: insufficient bytes"));
            }
            Ok((input[1 + len_of_len..1 + len_of_len + len].to_vec(), 1 + len_of_len + len))
        }
        _ => Err(eyre!("RLP list encountered; Nitro segments should be strings")),
    }
}

/// Iteratively decode a concatenated stream of RLP string items.
/// Returns a Vec of decoded item bytes (each item is a “segment”).
fn rlp_decode_stream_of_strings(mut input: &[u8]) -> Result<Vec<Vec<u8>>> {
    println!("RLP decode stream of strings");
    let mut out = Vec::new();
    while !input.is_empty() {
        let (bytes, consumed) = rlp_decode_one_string(input)?;
        out.push(bytes);
        input = &input[consumed..];
    }
    Ok(out)
}

// ---------- Nitro segment and L2 message decoding ----------

/// Given RLP-decoded segment bytes, extract L2 messages (handling per-segment brotli).
/// Ignore delayed messages (kind 2) here; you can add L1 resolution if needed.
fn segments_to_l2_messages(segments: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>> {
    let mut l2_msgs = Vec::new();
    for seg in segments {
        if seg.is_empty() {
            continue;
        }
        let kind = seg[0];
        let payload = &seg[1..];
        match kind {
            BATCH_SEGMENT_KIND_L2_MESSAGE => {
                l2_msgs.push(payload.to_vec());
            }
            BATCH_SEGMENT_KIND_L2_MESSAGE_BROTLI => {
                let dec = try_brotli_decompress(payload)
                    .ok_or_else(|| eyre!("Failed to brotli-decompress L2 message segment"))?;
                l2_msgs.push(dec);
            }
            BATCH_SEGMENT_KIND_DELAYED_MESSAGES => {
                // Optional: resolve delayed L1 messages to synthetic L2 messages
                // using Bridge/Inbox events (like getDelayedTx in the TS repo).
                // For now, skip.
            }
            _ => {
                // Unknown segment kind; skip or error
            }
        }
    }
    Ok(l2_msgs)
}

/// Read a big-endian u64 (Nitro nested batch length prefixes are 8-byte BE).
fn read_u64_be_8(bytes: &[u8]) -> Result<usize> {
    if bytes.len() < 8 {
        return Err(eyre!("insufficient bytes for u64"));
    }
    let mut v: u64 = 0;
    for b in &bytes[0..8] {
        v = (v << 8) | (*b as u64);
    }
    Ok(v as usize)
}

/// Decode a single L2 message and collect raw Ethereum transactions into `out_txs`.
fn decode_l2_message(mut msg: &[u8], out_txs: &mut Vec<Vec<u8>>) -> Result<()> {
    if msg.is_empty() {
        return Ok(());
    }
    let kind = msg[0];
    msg = &msg[1..];

    match kind {
        L2_MESSAGE_KIND_SIGNED_TX => {
            // The remainder is a standard Ethereum tx (legacy/0x01/0x02).
            // Do not force 0x02; leave parsing to your existing tx decoder.
            out_txs.push(msg.to_vec());
        }
        L2_MESSAGE_KIND_BATCH => {
            // Nested frames: [8-byte BE length][sub-message]...
            let mut cur = msg;
            while !cur.is_empty() {
                if cur.len() < 8 {
                    return Err(eyre!("nested batch: truncated length prefix"));
                }
                let next_len = read_u64_be_8(cur)?;
                if next_len > MAX_L2_MESSAGE_SIZE as usize {
                    return Err(eyre!("nested batch: sub-message too large"));
                }
                if 8 + next_len > cur.len() {
                    return Err(eyre!("nested batch: sub-message would overflow buffer"));
                }
                let frame = &cur[8..8 + next_len];
                decode_l2_message(frame, out_txs)?;
                cur = &cur[8 + next_len..];
            }
        }
        DELAYED_MSG_TO_BE_ADDED => {
            // Synthetic wrapper used by some pipelines; optional to support.
            // You could hexlify(msg) if you want to track these.
        }
        _ => {
            // Unknown L2 message kind; skip or error
        }
    }
    Ok(())
}

/// High-level function: given the unpacked payload after Nitro header handling,
/// produce raw Ethereum transaction bytes (Vec<Vec<u8>>).
fn decode_nitro_payload_to_txs(payload: &[u8]) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Err(eyre!("empty payload"));
    }

    // Case A: DAS/AnyTrust header present (0x80 flag bit set)
    if payload[0] & DASMESSAGE_HEADER_FLAG != 0 {
        // Layout per Nitro: [0]=header(with flag), [1..33]=keyset hash, [33..65]=data hash
        // To proceed you must fetch the real batch bytes from DAC using data hash, base64-decode,
        // then call this function recursively on the fetched bytes.
        return Err(eyre!("DAS flagged payload: fetch from DAC using data hash, then decode"));
    }

    // Case B: Nitro brotli header 0x00 (as in this repo path)
    if payload[0] == BROTLI_MESSAGE_HEADER_BYTE {
        if payload.len() < 2 {
            return Err(eyre!("nitro brotli: payload too short"));
        }
        println!("trying to decompress the data using brotli");
        let compressed = &payload[1..];
        let decompressed = try_brotli_decompress(compressed)
            .ok_or_else(|| eyre!("nitro brotli: decompression failed"))?;

        // Decompressed is an RLP stream (concatenated RLP string items)
        let segments = rlp_decode_stream_of_strings(&decompressed)
            .wrap_err("RLP segment stream decode failed")?;
        let l2_msgs = segments_to_l2_messages(segments)?;
        let mut out = Vec::new();
        for m in l2_msgs {
            decode_l2_message(&m, &mut out)?;
        }
        return Ok(out);
    }

    // Case C: No explicit Nitro header:
    // Try RLP segment stream directly (already decompressed)
    if let Ok(segments) = rlp_decode_stream_of_strings(payload) {
        println!("RLP segment stream decode successful");
        let l2_msgs = segments_to_l2_messages(segments)?;
        let mut out = Vec::new();
        for m in l2_msgs {
            decode_l2_message(&m, &mut out)?;
        }
        return Ok(out);
    }

    // Case D: Nested L2 frame at top-level:
    // Interpret [0..8] as big-endian length, then expect an L2 message starting at offset 8.
    if payload.len() >= 9 {
        println!("Nested L2 frame at top-level");
        if let Ok(next_len) = read_u64_be_8(payload) {
            if 8 + next_len <= payload.len() {
                let msg = &payload[8..8 + next_len];
                let mut out = Vec::new();
                decode_l2_message(msg, &mut out)?;
                return Ok(out);
            }
        }
    }

    Err(eyre!("unrecognized payload layout; cannot decode"))
}

// ---------- Public entry for your pipeline ----------

/// Process a raw EIP-4844 blob (131072 bytes) and return raw Ethereum txs found inside.
pub fn process_arbitrum_blob_to_txs(raw_blob: &[u8]) -> Result<Vec<Vec<u8>>> {
    // 1) Unpack 31-bytes-per-32-byte-field
    let mut payload = extract_payload_31_per_fe(raw_blob)
        .ok_or_else(|| eyre!("blob not 131072 bytes; cannot unpack"))?;
    
    println!("payload length: {}", payload.len());

    // 2) Optional: trim trailing zeros (common padding)
    // payload = trim_trailing_zeros(payload);

    // 3) Decode Nitro payload into raw Ethereum txs
    let txs = decode_nitro_payload_to_txs(&payload)?;
    Ok(txs)
}

pub fn process_blob_payload(blob: &[u8]) {
    // Step 1: rebuild payload from 31 bytes per field element
    println!("Processing blob of size: {}", blob.len());
    if let Some(payload) = extract_payload_31_per_fe(blob) {
        println!("reconstructed_payload_len: {}", payload.len());

        let payload_first_byte = &payload[0];
        // println!("Payload first byte in Hex: {}", justHex::encode(vec![*payload_first_byte]));
        // println!("Payload first byte in Decimal: {}", payload_first_byte);

        match u8::from_str_radix(&justHex::encode(vec![*payload_first_byte]), 16).unwrap() {
            DASMESSAGE_HEADER_FLAG => {
                println!("Payload first byte is DASMESSAGE_HEADER_FLAG");
            }
            BROTLI_MESSAGE_HEADER_BYTE => {
                println!("Payload first byte is BROTLI_MESSAGE_HEADER_BYTE");
            }
            _ => {
                println!("Payload first byte is not DASMESSAGE_HEADER_FLAG");
            }
        }



        // If you know the app-specific framing (SSZ, RLP, protobuf), parse `trimmed` accordingly here.
    } else {
        println!("Blob is not the canonical 131072-byte size; cannot apply 31-byte-per-FE extraction.");
    }
}

pub async fn ArbitrumParser(blob: Vec<u8>) -> Result<()> {
    // Replace these with your real data sources.
    // 1) You already have your blob from an API:
    // let blob: Vec<u8> = {
    //     // Example data; replace with your real Vec<u8>
    //     // e.g., let blob = api_client.fetch_blob(...).await?;
    //     vec![0u8; 64] // 2 words of 32 bytes each
    // };

    // 2) Optional fields (provide them if you have them; otherwise keep as None)
    let commitment: Option<[u8; 48]> = None; // e.g., Some([...;48])
    let proof: Option<[u8; 48]> = None;
    let tx_hash: Option<[u8; 32]> = None;
    let index: Option<u32> = None;

    // Process the blob
    let preview_words = 4;
    let (blob_len_bytes, words_32b, is_exact_eip4844_size, first_words) =
        process_blob(&blob, preview_words);

    println!("blob_len_bytes: {}", blob_len_bytes);
    println!("words_32b: {}", words_32b);
    println!("is_exact_eip4844_size: {}", is_exact_eip4844_size);

    for (i, w) in first_words.iter().enumerate() {
        println!("first_words[{i}]: {w}");
    }

    // If you have a commitment, compute versioned hash like in the project
    if let Some(cmt) = commitment.as_ref() {
        let versioned_hash = compute_versioned_hash_from_commitment(cmt);
        println!("commitment: {}", to_0x_hex(cmt));
        println!("versioned_hash: {}", versioned_hash);
    }

    // Optionally print proof
    if let Some(p) = proof.as_ref() {
        println!("proof: {}", to_0x_hex(p));
    }

    // Optionally print tx_hash and index
    if let Some(th) = tx_hash.as_ref() {
        // Use alloy's B256 to format consistently with 0x-prefix
        let b = B256::from_slice(th);
        println!("tx_hash: {b:#x}");
    }

    if let Some(i) = index {
        println!("index: {i}");
    }
    println!("====================================== PROCESSING BLOB PAYLOAD ======================================");
    process_arbitrum_blob_to_txs(&blob);
    Ok(())
}