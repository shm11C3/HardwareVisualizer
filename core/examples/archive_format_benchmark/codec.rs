//! Bounded codec candidates for the G1 benchmark experiment in issue #2052.
//! This module gathers format evidence; its bytes are not a committed production format.

use std::collections::HashMap;
use std::io::Write;

use flate2::{
  Compression as FlateCompression, Decompress, FlushDecompress, Status,
  write::DeflateEncoder,
};
use sha2::{Digest, Sha256};

pub const MAX_ROWS: usize = 4_096;
pub const MAX_COLUMNS: usize = 32;
pub const MAX_DECODED_BYTES: usize = 4 * 1024 * 1024;

const MAGIC: &[u8; 4] = b"HVA1";
const VERSION: u8 = 1;
const HEADER_WITHOUT_DIGEST_LEN: usize = 28;
const DIGEST_LEN: usize = 32;
const HEADER_LEN: usize = HEADER_WITHOUT_DIGEST_LEN + DIGEST_LEN;
const MAX_CELLS: usize = MAX_ROWS * MAX_COLUMNS;
const MAX_BODY_BYTES: usize = MAX_DECODED_BYTES + MAX_CELLS * 6 + MAX_COLUMNS * 3;
const MAX_STORED_BYTES: usize = MAX_BODY_BYTES + 64 * 1024;

const TAG_NULL: u8 = 0;
const TAG_INTEGER: u8 = 1;
const TAG_REAL: u8 = 2;
const TAG_TEXT: u8 = 3;
const TAG_BLOB: u8 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
  Null,
  Integer(i64),
  Real(u64),
  Text(Vec<u8>),
  Blob(Vec<u8>),
}

pub type Record = Vec<Value>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Layout {
  Row,
  Columnar,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Compression {
  None,
  Deflate,
}

pub fn encode(
  records: &[Record],
  layout: Layout,
  compression: Compression,
) -> Result<Vec<u8>, String> {
  let (column_count, decoded_bytes) = validate_records(records)?;
  let mut body = Vec::new();
  match layout {
    Layout::Row => encode_rows(records, &mut body),
    Layout::Columnar => encode_columns(records, column_count, &mut body)?,
  }
  if body.len() > MAX_BODY_BYTES {
    return Err("encoded body exceeds bounded framing limit".into());
  }
  let body_len = body.len();

  let stored = match compression {
    Compression::None => body,
    Compression::Deflate => {
      let mut encoder = DeflateEncoder::new(Vec::new(), FlateCompression::default());
      encoder
        .write_all(&body)
        .map_err(|error| format!("deflate encode failed: {error}"))?;
      encoder
        .finish()
        .map_err(|error| format!("deflate encode failed: {error}"))?
    }
  };
  if stored.len() > MAX_STORED_BYTES {
    return Err("stored payload exceeds bounded framing limit".into());
  }

  let mut header = Vec::with_capacity(HEADER_WITHOUT_DIGEST_LEN);
  header.extend_from_slice(MAGIC);
  header.push(VERSION);
  header.push(layout_flag(layout));
  header.push(compression_flag(compression));
  header.push(0);
  push_u32(&mut header, records.len(), "row count")?;
  push_u32(&mut header, column_count, "column count")?;
  push_u32(&mut header, decoded_bytes, "decoded byte count")?;
  push_u32(&mut header, body_len, "body byte count")?;
  push_u32(&mut header, stored.len(), "stored byte count")?;

  let digest = digest(&header, &stored);
  let mut framed = Vec::with_capacity(HEADER_LEN + stored.len());
  framed.extend_from_slice(&header);
  framed.extend_from_slice(&digest);
  framed.extend_from_slice(&stored);
  Ok(framed)
}

pub fn decode(payload: &[u8]) -> Result<Vec<Record>, String> {
  if payload.len() < HEADER_LEN {
    return Err("payload is truncated before the header ends".into());
  }
  if &payload[..4] != MAGIC {
    return Err("unknown archive payload magic".into());
  }
  if payload[4] != VERSION {
    return Err(format!(
      "unsupported archive payload version {}",
      payload[4]
    ));
  }
  let layout = parse_layout(payload[5])?;
  let compression = parse_compression(payload[6])?;
  if payload[7] != 0 {
    return Err("non-zero reserved header flags".into());
  }

  let row_count = header_u32(payload, 8)?;
  let column_count = header_u32(payload, 12)?;
  let declared_decoded_bytes = header_u32(payload, 16)?;
  let body_len = header_u32(payload, 20)?;
  let stored_len = header_u32(payload, 24)?;
  if row_count > MAX_ROWS {
    return Err(format!("row count exceeds {MAX_ROWS} row limit"));
  }
  if column_count > MAX_COLUMNS {
    return Err(format!("column count exceeds {MAX_COLUMNS} column limit"));
  }
  if declared_decoded_bytes > MAX_DECODED_BYTES {
    return Err(format!(
      "declared decoded size exceeds {MAX_DECODED_BYTES} byte limit"
    ));
  }
  if body_len > MAX_BODY_BYTES {
    return Err("encoded body exceeds bounded framing limit".into());
  }
  if stored_len > MAX_STORED_BYTES {
    return Err("stored payload exceeds bounded framing limit".into());
  }
  let expected_len = HEADER_LEN
    .checked_add(stored_len)
    .ok_or_else(|| "payload length overflows address space".to_owned())?;
  if payload.len() != expected_len {
    return Err(if payload.len() < expected_len {
      "payload is truncated".into()
    } else {
      "payload has trailing bytes".into()
    });
  }

  let expected_digest = &payload[HEADER_WITHOUT_DIGEST_LEN..HEADER_LEN];
  let stored = &payload[HEADER_LEN..];
  if digest(&payload[..HEADER_WITHOUT_DIGEST_LEN], stored) != expected_digest {
    return Err("archive payload integrity check failed".into());
  }

  let body = match compression {
    Compression::None => {
      if stored.len() != body_len {
        return Err("uncompressed body length does not match header".into());
      }
      stored.to_vec()
    }
    Compression::Deflate => inflate_bounded(stored, body_len)?,
  };
  let mut reader = Reader::new(&body);
  let records = match layout {
    Layout::Row => {
      decode_rows(&mut reader, row_count, column_count, declared_decoded_bytes)?
    }
    Layout::Columnar => {
      decode_columns(&mut reader, row_count, column_count, declared_decoded_bytes)?
    }
  };
  if !reader.is_finished() {
    return Err("decoded body has trailing bytes".into());
  }
  Ok(records)
}

fn validate_records(records: &[Record]) -> Result<(usize, usize), String> {
  if records.len() > MAX_ROWS {
    return Err(format!("record count exceeds {MAX_ROWS} row limit"));
  }
  let column_count = records.first().map_or(0, Vec::len);
  if column_count > MAX_COLUMNS {
    return Err(format!("record width exceeds {MAX_COLUMNS} column limit"));
  }
  let mut decoded_bytes = 0usize;
  for record in records {
    if record.len() != column_count {
      return Err("records do not have a uniform column count".into());
    }
    for value in record {
      let value_bytes = decoded_value_size(value)?;
      decoded_bytes = decoded_bytes
        .checked_add(value_bytes)
        .ok_or_else(|| "decoded size overflows address space".to_owned())?;
      if decoded_bytes > MAX_DECODED_BYTES {
        return Err(format!(
          "records exceed {MAX_DECODED_BYTES} decoded byte limit"
        ));
      }
    }
  }
  Ok((column_count, decoded_bytes))
}

fn decoded_value_size(value: &Value) -> Result<usize, String> {
  match value {
    Value::Null => Ok(1),
    Value::Integer(_) | Value::Real(_) => Ok(9),
    Value::Text(bytes) | Value::Blob(bytes) => 1usize
      .checked_add(bytes.len())
      .ok_or_else(|| "decoded size overflows address space".to_owned()),
  }
}

fn encode_rows(records: &[Record], output: &mut Vec<u8>) {
  for record in records {
    for value in record {
      match value {
        Value::Null => output.push(TAG_NULL),
        Value::Integer(value) => {
          output.push(TAG_INTEGER);
          output.extend_from_slice(&value.to_le_bytes());
        }
        Value::Real(bits) => {
          output.push(TAG_REAL);
          output.extend_from_slice(&bits.to_le_bytes());
        }
        Value::Text(bytes) => {
          output.push(TAG_TEXT);
          write_varint(bytes.len() as u64, output);
          output.extend_from_slice(bytes);
        }
        Value::Blob(bytes) => {
          output.push(TAG_BLOB);
          write_varint(bytes.len() as u64, output);
          output.extend_from_slice(bytes);
        }
      }
    }
  }
}

fn encode_columns(
  records: &[Record],
  column_count: usize,
  output: &mut Vec<u8>,
) -> Result<(), String> {
  for column in 0..column_count {
    let mut dictionary = Vec::<Vec<u8>>::new();
    let mut dictionary_ids = HashMap::<Vec<u8>, u32>::new();
    for record in records {
      output.push(tag(&record[column]));
      if let Value::Text(bytes) = &record[column]
        && !dictionary_ids.contains_key(bytes)
      {
        let id = u32::try_from(dictionary.len())
          .map_err(|_| "text dictionary has too many entries".to_owned())?;
        dictionary_ids.insert(bytes.clone(), id);
        dictionary.push(bytes.clone());
      }
    }
    write_varint(dictionary.len() as u64, output);
    for entry in &dictionary {
      write_varint(entry.len() as u64, output);
      output.extend_from_slice(entry);
    }

    let mut previous_integer = 0i64;
    let mut previous_real = 0u64;
    for record in records {
      match &record[column] {
        Value::Null => {}
        Value::Integer(value) => {
          if let Some(delta) = value.checked_sub(previous_integer) {
            output.push(0);
            write_varint(zigzag_encode(delta), output);
          } else {
            output.push(1);
            output.extend_from_slice(&value.to_le_bytes());
          }
          previous_integer = *value;
        }
        Value::Real(bits) => {
          write_varint(*bits ^ previous_real, output);
          previous_real = *bits;
        }
        Value::Text(bytes) => {
          let id = dictionary_ids
            .get(bytes)
            .ok_or_else(|| "text dictionary construction failed".to_owned())?;
          write_varint(u64::from(*id), output);
        }
        Value::Blob(bytes) => {
          write_varint(bytes.len() as u64, output);
          output.extend_from_slice(bytes);
        }
      }
    }
  }
  Ok(())
}

fn decode_rows(
  reader: &mut Reader<'_>,
  row_count: usize,
  column_count: usize,
  declared_decoded_bytes: usize,
) -> Result<Vec<Record>, String> {
  let mut budget = DecodeBudget::new(declared_decoded_bytes);
  let mut records = Vec::with_capacity(row_count);
  for _ in 0..row_count {
    let mut record = Vec::with_capacity(column_count);
    for _ in 0..column_count {
      let value = match reader.read_u8("value tag")? {
        TAG_NULL => Value::Null,
        TAG_INTEGER => Value::Integer(i64::from_le_bytes(reader.read_array("integer")?)),
        TAG_REAL => Value::Real(u64::from_le_bytes(reader.read_array("real")?)),
        TAG_TEXT => Value::Text(reader.read_sized_bytes("text")?),
        TAG_BLOB => Value::Blob(reader.read_sized_bytes("blob")?),
        unknown => return Err(format!("unknown value tag {unknown}")),
      };
      budget.add(&value)?;
      record.push(value);
    }
    records.push(record);
  }
  budget.finish()?;
  Ok(records)
}

fn decode_columns(
  reader: &mut Reader<'_>,
  row_count: usize,
  column_count: usize,
  declared_decoded_bytes: usize,
) -> Result<Vec<Record>, String> {
  let mut records = (0..row_count)
    .map(|_| Vec::with_capacity(column_count))
    .collect::<Vec<_>>();
  let mut budget = DecodeBudget::new(declared_decoded_bytes);
  for _ in 0..column_count {
    let tags = reader.read_exact(row_count, "column type tags")?.to_vec();
    for tag in &tags {
      if *tag > TAG_BLOB {
        return Err(format!("unknown value tag {tag}"));
      }
    }

    let dictionary_count = reader.read_bounded_varint(MAX_ROWS, "dictionary count")?;
    let mut dictionary = Vec::with_capacity(dictionary_count);
    let mut dictionary_bytes = 0usize;
    for _ in 0..dictionary_count {
      let entry = reader.read_sized_bytes("dictionary entry")?;
      dictionary_bytes = dictionary_bytes
        .checked_add(entry.len())
        .ok_or_else(|| "dictionary byte count overflows address space".to_owned())?;
      if dictionary_bytes > declared_decoded_bytes {
        return Err("text dictionary exceeds declared decoded size".into());
      }
      dictionary.push(entry);
    }

    let mut previous_integer = 0i64;
    let mut previous_real = 0u64;
    for (row, tag) in tags.into_iter().enumerate() {
      let value = match tag {
        TAG_NULL => Value::Null,
        TAG_INTEGER => {
          let value = match reader.read_u8("integer delta marker")? {
            0 => {
              let delta = zigzag_decode(reader.read_varint("integer delta")?);
              previous_integer
                .checked_add(delta)
                .ok_or_else(|| "integer delta overflows i64".to_owned())?
            }
            1 => i64::from_le_bytes(reader.read_array("absolute integer")?),
            marker => return Err(format!("unknown integer delta marker {marker}")),
          };
          previous_integer = value;
          Value::Integer(value)
        }
        TAG_REAL => {
          let bits = previous_real ^ reader.read_varint("real XOR")?;
          previous_real = bits;
          Value::Real(bits)
        }
        TAG_TEXT => {
          let id = reader.read_bounded_varint(dictionary.len(), "text dictionary id")?;
          let bytes = dictionary
            .get(id)
            .ok_or_else(|| "text dictionary id is out of range".to_owned())?
            .clone();
          Value::Text(bytes)
        }
        TAG_BLOB => Value::Blob(reader.read_sized_bytes("blob")?),
        _ => unreachable!("tags were validated above"),
      };
      budget.add(&value)?;
      records[row].push(value);
    }
  }
  budget.finish()?;
  Ok(records)
}

struct DecodeBudget {
  expected: usize,
  used: usize,
}

impl DecodeBudget {
  fn new(expected: usize) -> Self {
    Self { expected, used: 0 }
  }

  fn add(&mut self, value: &Value) -> Result<(), String> {
    self.used = self
      .used
      .checked_add(decoded_value_size(value)?)
      .ok_or_else(|| "decoded size overflows address space".to_owned())?;
    if self.used > self.expected || self.used > MAX_DECODED_BYTES {
      return Err("decoded records exceed declared decoded size".into());
    }
    Ok(())
  }

  fn finish(self) -> Result<(), String> {
    if self.used != self.expected {
      return Err("decoded record size does not match header".into());
    }
    Ok(())
  }
}

struct Reader<'a> {
  bytes: &'a [u8],
  offset: usize,
}

impl<'a> Reader<'a> {
  fn new(bytes: &'a [u8]) -> Self {
    Self { bytes, offset: 0 }
  }

  fn read_u8(&mut self, what: &str) -> Result<u8, String> {
    Ok(self.read_exact(1, what)?[0])
  }

  fn read_array<const N: usize>(&mut self, what: &str) -> Result<[u8; N], String> {
    self
      .read_exact(N, what)?
      .try_into()
      .map_err(|_| format!("invalid {what} width"))
  }

  fn read_exact(&mut self, len: usize, what: &str) -> Result<&'a [u8], String> {
    let end = self
      .offset
      .checked_add(len)
      .ok_or_else(|| format!("{what} length overflows address space"))?;
    let bytes = self
      .bytes
      .get(self.offset..end)
      .ok_or_else(|| format!("payload is truncated while reading {what}"))?;
    self.offset = end;
    Ok(bytes)
  }

  fn read_varint(&mut self, what: &str) -> Result<u64, String> {
    let mut value = 0u64;
    for index in 0..10 {
      let byte = self.read_u8(what)?;
      if index == 9 && byte > 1 {
        return Err(format!("{what} varint overflows u64"));
      }
      value |= u64::from(byte & 0x7f) << (index * 7);
      if byte & 0x80 == 0 {
        return Ok(value);
      }
    }
    Err(format!("{what} varint is too long"))
  }

  fn read_bounded_varint(&mut self, max: usize, what: &str) -> Result<usize, String> {
    let value = self.read_varint(what)?;
    let value =
      usize::try_from(value).map_err(|_| format!("{what} exceeds address space"))?;
    if value > max {
      return Err(format!("{what} exceeds limit {max}"));
    }
    Ok(value)
  }

  fn read_sized_bytes(&mut self, what: &str) -> Result<Vec<u8>, String> {
    let len = self.read_bounded_varint(MAX_DECODED_BYTES, &format!("{what} length"))?;
    Ok(self.read_exact(len, what)?.to_vec())
  }

  fn is_finished(&self) -> bool {
    self.offset == self.bytes.len()
  }
}

fn inflate_bounded(stored: &[u8], expected_len: usize) -> Result<Vec<u8>, String> {
  let output_limit = expected_len
    .checked_add(1)
    .ok_or_else(|| "inflated body length overflows address space".to_owned())?;
  let mut body = vec![0; output_limit];
  let mut decoder = Decompress::new(false);
  let mut previous_in = 0;
  let mut previous_out = 0;
  loop {
    let input_offset = decoder.total_in() as usize;
    let output_offset = decoder.total_out() as usize;
    let status = decoder
      .decompress(
        &stored[input_offset..],
        &mut body[output_offset..],
        FlushDecompress::Finish,
      )
      .map_err(|error| format!("deflate decode failed: {error}"))?;
    if decoder.total_out() as usize > expected_len {
      return Err("inflated body exceeds declared body length".into());
    }
    if status == Status::StreamEnd {
      break;
    }
    if decoder.total_in() == previous_in && decoder.total_out() == previous_out {
      return Err("deflate stream ended before its end marker".into());
    }
    previous_in = decoder.total_in();
    previous_out = decoder.total_out();
  }
  if decoder.total_out() as usize != expected_len {
    return Err("inflated body length does not match header".into());
  }
  if decoder.total_in() as usize != stored.len() {
    return Err("deflate stream has trailing bytes".into());
  }
  body.truncate(expected_len);
  Ok(body)
}

fn tag(value: &Value) -> u8 {
  match value {
    Value::Null => TAG_NULL,
    Value::Integer(_) => TAG_INTEGER,
    Value::Real(_) => TAG_REAL,
    Value::Text(_) => TAG_TEXT,
    Value::Blob(_) => TAG_BLOB,
  }
}

fn zigzag_encode(value: i64) -> u64 {
  ((value << 1) ^ (value >> 63)) as u64
}

fn zigzag_decode(value: u64) -> i64 {
  ((value >> 1) as i64) ^ -((value & 1) as i64)
}

fn write_varint(mut value: u64, output: &mut Vec<u8>) {
  while value >= 0x80 {
    output.push((value as u8 & 0x7f) | 0x80);
    value >>= 7;
  }
  output.push(value as u8);
}

fn push_u32(output: &mut Vec<u8>, value: usize, what: &str) -> Result<(), String> {
  let value = u32::try_from(value).map_err(|_| format!("{what} exceeds u32"))?;
  output.extend_from_slice(&value.to_le_bytes());
  Ok(())
}

fn header_u32(payload: &[u8], offset: usize) -> Result<usize, String> {
  let bytes: [u8; 4] = payload
    .get(offset..offset + 4)
    .ok_or_else(|| "payload is truncated in numeric header".to_owned())?
    .try_into()
    .map_err(|_| "invalid numeric header width".to_owned())?;
  Ok(u32::from_le_bytes(bytes) as usize)
}

fn digest(header: &[u8], stored: &[u8]) -> [u8; DIGEST_LEN] {
  let mut hasher = Sha256::new();
  hasher.update(header);
  hasher.update(stored);
  hasher.finalize().into()
}

fn layout_flag(layout: Layout) -> u8 {
  match layout {
    Layout::Row => 0,
    Layout::Columnar => 1,
  }
}

fn compression_flag(compression: Compression) -> u8 {
  match compression {
    Compression::None => 0,
    Compression::Deflate => 1,
  }
}

fn parse_layout(flag: u8) -> Result<Layout, String> {
  match flag {
    0 => Ok(Layout::Row),
    1 => Ok(Layout::Columnar),
    _ => Err(format!("unknown layout flag {flag}")),
  }
}

fn parse_compression(flag: u8) -> Result<Compression, String> {
  match flag {
    0 => Ok(Compression::None),
    1 => Ok(Compression::Deflate),
    _ => Err(format!("unknown compression flag {flag}")),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn resign(frame: &mut [u8]) {
    let signed = digest(&frame[..HEADER_WITHOUT_DIGEST_LEN], &frame[HEADER_LEN..]);
    frame[HEADER_WITHOUT_DIGEST_LEN..HEADER_LEN].copy_from_slice(&signed);
  }

  fn replace_stored(frame: &mut Vec<u8>, stored: &[u8], body_len: usize) {
    frame.truncate(HEADER_LEN);
    frame.extend_from_slice(stored);
    frame[20..24].copy_from_slice(&(body_len as u32).to_le_bytes());
    frame[24..28].copy_from_slice(&(stored.len() as u32).to_le_bytes());
    resign(frame);
  }

  fn round_trip(records: &[Record]) {
    for layout in [Layout::Row, Layout::Columnar] {
      for compression in [Compression::None, Compression::Deflate] {
        let encoded = encode(records, layout, compression).unwrap();
        assert_eq!(decode(&encoded).unwrap(), records);
      }
    }
  }

  fn edge_records() -> Vec<Record> {
    vec![
      vec![
        Value::Integer(i64::MIN),
        Value::Real(f64::NAN.to_bits()),
        Value::Text("温度🌡️".as_bytes().to_vec()),
        Value::Blob(vec![0, 0xff, 0x80]),
        Value::Null,
      ],
      vec![
        Value::Integer(i64::MAX),
        Value::Real((-0.0f64).to_bits()),
        Value::Text(vec![0xff, 0, b'a']),
        Value::Blob(Vec::new()),
        Value::Integer(-1),
      ],
      vec![
        Value::Integer(i64::MIN),
        Value::Real(f64::INFINITY.to_bits()),
        Value::Text("温度🌡️".as_bytes().to_vec()),
        Value::Blob(vec![0, 0xff, 0x80]),
        Value::Null,
      ],
    ]
  }

  #[test]
  fn round_trips_storage_classes_and_exact_numeric_bits() {
    round_trip(&edge_records());
  }

  #[test]
  fn preserves_duplicate_and_empty_records() {
    let duplicate = vec![Value::Null, Value::Text(b"same".to_vec())];
    round_trip(&[duplicate.clone(), duplicate]);
    round_trip(&[vec![], vec![], vec![]]);
    round_trip(&[]);
  }

  #[test]
  fn rejects_non_uniform_and_public_limits() {
    assert!(
      encode(&[vec![], vec![Value::Null]], Layout::Row, Compression::None).is_err()
    );
    assert!(encode(&vec![vec![]; MAX_ROWS + 1], Layout::Row, Compression::None).is_err());
    assert!(
      encode(
        &[vec![Value::Null; MAX_COLUMNS + 1]],
        Layout::Row,
        Compression::None
      )
      .is_err()
    );
    assert!(
      encode(
        &[vec![Value::Blob(vec![0; MAX_DECODED_BYTES])]],
        Layout::Row,
        Compression::None
      )
      .is_err()
    );
  }

  #[test]
  fn accepts_exact_row_and_decoded_byte_limits() {
    round_trip(&vec![vec![]; MAX_ROWS]);
    round_trip(&[vec![Value::Blob(vec![0; MAX_DECODED_BYTES - 1])]]);
  }

  #[test]
  fn rejects_corrupt_truncated_trailing_and_unknown_framing() {
    let encoded =
      encode(&edge_records(), Layout::Columnar, Compression::Deflate).unwrap();

    let mut corrupt = encoded.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(decode(&corrupt).is_err());
    assert!(decode(&encoded[..encoded.len() - 1]).is_err());
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(decode(&trailing).is_err());

    for offset in [4, 5, 6, 7] {
      let mut unknown = encoded.clone();
      unknown[offset] = 0xff;
      assert!(decode(&unknown).is_err());
    }
  }

  #[test]
  fn rejects_adversarial_counts_and_lengths_before_allocation() {
    let encoded = encode(&[vec![Value::Null]], Layout::Row, Compression::None).unwrap();
    for (offset, value) in [
      (8, (MAX_ROWS + 1) as u32),
      (12, (MAX_COLUMNS + 1) as u32),
      (16, (MAX_DECODED_BYTES + 1) as u32),
      (20, (MAX_BODY_BYTES + 1) as u32),
    ] {
      let mut adversarial = encoded.clone();
      adversarial[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
      assert!(decode(&adversarial).is_err());
    }
  }

  #[test]
  fn rejects_signed_invalid_dictionary_id() {
    let mut encoded = encode(
      &[vec![Value::Text(b"a".to_vec())]],
      Layout::Columnar,
      Compression::None,
    )
    .unwrap();
    *encoded.last_mut().unwrap() = 1;
    resign(&mut encoded);
    assert!(decode(&encoded).is_err());
  }

  #[test]
  fn rejects_signed_overflowing_varint() {
    let mut encoded =
      encode(&[vec![Value::Real(0)]], Layout::Columnar, Compression::None).unwrap();
    let body = [
      TAG_REAL, 0, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
    ];
    replace_stored(&mut encoded, &body, body.len());
    assert!(decode(&encoded).is_err());
  }

  #[test]
  fn rejects_signed_decoded_byte_budget_overrun() {
    let mut encoded =
      encode(&[vec![Value::Null]], Layout::Row, Compression::None).unwrap();
    encoded[16..20].copy_from_slice(&0u32.to_le_bytes());
    resign(&mut encoded);
    assert!(decode(&encoded).is_err());
  }

  #[test]
  fn rejects_deflate_expansion_past_global_limit() {
    let mut encoder = DeflateEncoder::new(Vec::new(), FlateCompression::default());
    encoder.write_all(&vec![0; MAX_BODY_BYTES + 1]).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut encoded = encode(&[], Layout::Row, Compression::Deflate).unwrap();
    replace_stored(&mut encoded, &compressed, MAX_BODY_BYTES);
    assert!(decode(&encoded).is_err());
  }

  #[test]
  fn rejects_signed_truncated_deflate_stream() {
    let mut encoded = encode(&[], Layout::Row, Compression::Deflate).unwrap();
    encoded.pop();
    let stored_len = encoded.len() - HEADER_LEN;
    encoded[24..28].copy_from_slice(&(stored_len as u32).to_le_bytes());
    resign(&mut encoded);
    assert!(decode(&encoded).is_err());
  }

  #[test]
  fn row_encoding_has_stable_golden_bytes_without_digest() {
    let encoded = encode(
      &[vec![
        Value::Null,
        Value::Integer(-1),
        Value::Real(0x3ff0_0000_0000_0000),
        Value::Text(b"a".to_vec()),
        Value::Blob(vec![0xff]),
      ]],
      Layout::Row,
      Compression::None,
    )
    .unwrap();
    assert_eq!(
      &encoded[..HEADER_WITHOUT_DIGEST_LEN],
      &[
        b'H', b'V', b'A', b'1', 1, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0, 23, 0, 0, 0, 25, 0,
        0, 0, 25, 0, 0, 0,
      ]
    );
    assert_eq!(
      &encoded[HEADER_LEN..],
      &[
        0, 1, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 2, 0, 0, 0, 0, 0, 0, 0xf0,
        0x3f, 3, 1, b'a', 4, 1, 0xff,
      ]
    );
  }
}
