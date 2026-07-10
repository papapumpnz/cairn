use half::f16;
use std::fmt;

/// CAIRN's closed canonical-CBOR value profile.
///
/// Maps are represented in arbitrary insertion order and sorted by encoded key
/// length then bytewise key encoding. Duplicate canonical keys are rejected.
/// Floats use the shortest exact IEEE width; all NaNs normalize to one half
/// precision quiet NaN. Indefinite lengths and CBOR tags are unsupported.
#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalValue {
    Null,
    Bool(bool),
    Unsigned(u64),
    Negative(i64),
    Float(f64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Self>),
    Map(Vec<(Self, Self)>),
}

/// Decode resource limits applied before allocation or recursion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanonicalLimits {
    pub max_input_bytes: usize,
    pub max_depth: usize,
    pub max_collection_items: usize,
    pub max_scalar_bytes: usize,
}

impl Default for CanonicalLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_depth: 128,
            max_collection_items: 1_000_000,
            max_scalar_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Canonical encoding failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalError {
    InputTooLarge { actual: usize, maximum: usize },
    Truncated { offset: usize },
    TrailingBytes { offset: usize },
    DepthLimit { maximum: usize },
    CollectionLimit { actual: u64, maximum: usize },
    ScalarLimit { actual: u64, maximum: usize },
    LengthOverflow,
    InvalidUtf8 { offset: usize },
    InvalidNegative(i64),
    IntegerOutOfRange,
    UnsupportedType { major: u8, additional: u8 },
    IndefiniteLength,
    DuplicateMapKey,
    NonCanonical,
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge { actual, maximum } => {
                write!(formatter, "input has {actual} bytes; maximum is {maximum}")
            }
            Self::Truncated { offset } => write!(formatter, "truncated CBOR at byte {offset}"),
            Self::TrailingBytes { offset } => write!(formatter, "trailing bytes begin at {offset}"),
            Self::DepthLimit { maximum } => write!(formatter, "decode depth exceeds {maximum}"),
            Self::CollectionLimit { actual, maximum } => {
                write!(
                    formatter,
                    "collection has {actual} items; maximum is {maximum}"
                )
            }
            Self::ScalarLimit { actual, maximum } => {
                write!(formatter, "scalar has {actual} bytes; maximum is {maximum}")
            }
            Self::LengthOverflow => formatter.write_str("decoded length exceeds address space"),
            Self::InvalidUtf8 { offset } => write!(formatter, "invalid UTF-8 at byte {offset}"),
            Self::InvalidNegative(value) => write!(formatter, "negative variant contains {value}"),
            Self::IntegerOutOfRange => formatter.write_str("negative integer is outside i64"),
            Self::UnsupportedType { major, additional } => {
                write!(
                    formatter,
                    "unsupported CBOR major {major}, additional {additional}"
                )
            }
            Self::IndefiniteLength => formatter.write_str("indefinite-length CBOR is prohibited"),
            Self::DuplicateMapKey => formatter.write_str("map has duplicate canonical key"),
            Self::NonCanonical => {
                formatter.write_str("input is valid CBOR but not canonical CAIRN CBOR")
            }
        }
    }
}

impl std::error::Error for CanonicalError {}

/// Deterministic canonical byte producer.
pub trait CanonicalEncode {
    /// Produce the complete canonical-CBOR representation.
    ///
    /// # Errors
    ///
    /// Returns a typed error for invalid values or duplicate canonical keys.
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError>;
}

/// Strict canonical byte consumer.
pub trait CanonicalDecode: Sized {
    /// Decode one complete value under explicit resource bounds.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-canonical, trailing, or over-limit input.
    fn decode_canonical(bytes: &[u8], limits: CanonicalLimits) -> Result<Self, CanonicalError>;
}

impl CanonicalEncode for CanonicalValue {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        let mut output = Vec::new();
        encode_value(self, &mut output)?;
        Ok(output)
    }
}

impl CanonicalDecode for CanonicalValue {
    fn decode_canonical(bytes: &[u8], limits: CanonicalLimits) -> Result<Self, CanonicalError> {
        decode_canonical(bytes, limits)
    }
}

/// Decode and re-encode one value so non-preferred widths and map order fail.
///
/// # Errors
///
/// Returns a precise syntax, bound, or canonicality error.
pub fn decode_canonical(
    bytes: &[u8],
    limits: CanonicalLimits,
) -> Result<CanonicalValue, CanonicalError> {
    if bytes.len() > limits.max_input_bytes {
        return Err(CanonicalError::InputTooLarge {
            actual: bytes.len(),
            maximum: limits.max_input_bytes,
        });
    }
    let mut decoder = Decoder {
        bytes,
        offset: 0,
        limits,
    };
    let value = decoder.decode_value(0)?;
    if decoder.offset != bytes.len() {
        return Err(CanonicalError::TrailingBytes {
            offset: decoder.offset,
        });
    }
    if value.encode_canonical()?.as_slice() != bytes {
        return Err(CanonicalError::NonCanonical);
    }
    Ok(value)
}

fn encode_value(value: &CanonicalValue, output: &mut Vec<u8>) -> Result<(), CanonicalError> {
    match value {
        CanonicalValue::Null => output.push(0xf6),
        CanonicalValue::Bool(false) => output.push(0xf4),
        CanonicalValue::Bool(true) => output.push(0xf5),
        CanonicalValue::Unsigned(number) => encode_major(0, *number, output),
        CanonicalValue::Negative(number) => {
            if *number >= 0 {
                return Err(CanonicalError::InvalidNegative(*number));
            }
            let magnitude = u64::try_from(-1_i128 - i128::from(*number))
                .map_err(|_| CanonicalError::IntegerOutOfRange)?;
            encode_major(1, magnitude, output);
        }
        CanonicalValue::Float(number) => encode_float(*number, output),
        CanonicalValue::Bytes(bytes) => {
            encode_major(2, u64_len(bytes.len())?, output);
            output.extend_from_slice(bytes);
        }
        CanonicalValue::Text(text) => {
            encode_major(3, u64_len(text.len())?, output);
            output.extend_from_slice(text.as_bytes());
        }
        CanonicalValue::Array(values) => {
            encode_major(4, u64_len(values.len())?, output);
            for item in values {
                encode_value(item, output)?;
            }
        }
        CanonicalValue::Map(entries) => encode_map(entries, output)?,
    }
    Ok(())
}

fn encode_map(
    entries: &[(CanonicalValue, CanonicalValue)],
    output: &mut Vec<u8>,
) -> Result<(), CanonicalError> {
    let mut encoded = entries
        .iter()
        .map(|(key, value)| Ok((key.encode_canonical()?, value)))
        .collect::<Result<Vec<_>, CanonicalError>>()?;
    encoded.sort_by(|(left, _), (right, _)| {
        left.len().cmp(&right.len()).then_with(|| left.cmp(right))
    });
    if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(CanonicalError::DuplicateMapKey);
    }
    encode_major(5, u64_len(encoded.len())?, output);
    for (key, value) in encoded {
        output.extend_from_slice(&key);
        encode_value(value, output)?;
    }
    Ok(())
}

fn encode_major(major: u8, value: u64, output: &mut Vec<u8>) {
    let bytes = value.to_be_bytes();
    match value {
        0..=23 => output.push((major << 5) | bytes[7]),
        24..=0xff => {
            output.push((major << 5) | 24);
            output.push(bytes[7]);
        }
        0x100..=0xffff => {
            output.push((major << 5) | 25);
            output.extend_from_slice(&bytes[6..]);
        }
        0x1_0000..=0xffff_ffff => {
            output.push((major << 5) | 26);
            output.extend_from_slice(&bytes[4..]);
        }
        _ => {
            output.push((major << 5) | 27);
            output.extend_from_slice(&bytes);
        }
    }
}

fn encode_float(value: f64, output: &mut Vec<u8>) {
    if value.is_nan() {
        output.extend_from_slice(&[0xf9, 0x7e, 0x00]);
        return;
    }
    let half = f16::from_f64(value);
    if same_float(half.to_f64(), value) {
        output.push(0xf9);
        output.extend_from_slice(&half.to_bits().to_be_bytes());
        return;
    }
    // The narrowing is accepted only when the following bit-exact widening
    // comparison proves that it did not change the represented value.
    #[allow(clippy::cast_possible_truncation)]
    let single = value as f32;
    if same_float(f64::from(single), value) {
        output.push(0xfa);
        output.extend_from_slice(&single.to_bits().to_be_bytes());
        return;
    }
    output.push(0xfb);
    output.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn same_float(left: f64, right: f64) -> bool {
    left.to_bits() == right.to_bits()
}

fn u64_len(value: usize) -> Result<u64, CanonicalError> {
    u64::try_from(value).map_err(|_| CanonicalError::LengthOverflow)
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
    limits: CanonicalLimits,
}

impl Decoder<'_> {
    fn decode_value(&mut self, depth: usize) -> Result<CanonicalValue, CanonicalError> {
        if depth > self.limits.max_depth {
            return Err(CanonicalError::DepthLimit {
                maximum: self.limits.max_depth,
            });
        }
        let initial = self.read_byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        match major {
            0 => Ok(CanonicalValue::Unsigned(self.read_argument(additional)?)),
            1 => {
                let magnitude = self.read_argument(additional)?;
                let number = -1_i128 - i128::from(magnitude);
                let number =
                    i64::try_from(number).map_err(|_| CanonicalError::IntegerOutOfRange)?;
                Ok(CanonicalValue::Negative(number))
            }
            2 => {
                let length = self.read_scalar_length(additional)?;
                Ok(CanonicalValue::Bytes(self.read_slice(length)?.to_vec()))
            }
            3 => {
                let length = self.read_scalar_length(additional)?;
                let start = self.offset;
                let bytes = self.read_slice(length)?;
                let text = std::str::from_utf8(bytes)
                    .map_err(|_| CanonicalError::InvalidUtf8 { offset: start })?;
                Ok(CanonicalValue::Text(text.to_owned()))
            }
            4 => {
                let length = self.read_collection_length(additional)?;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    values.push(self.decode_value(depth + 1)?);
                }
                Ok(CanonicalValue::Array(values))
            }
            5 => {
                let length = self.read_collection_length(additional)?;
                let mut entries = Vec::with_capacity(length);
                for _ in 0..length {
                    let key = self.decode_value(depth + 1)?;
                    let value = self.decode_value(depth + 1)?;
                    entries.push((key, value));
                }
                Ok(CanonicalValue::Map(entries))
            }
            7 => self.decode_simple(additional),
            _ => Err(CanonicalError::UnsupportedType { major, additional }),
        }
    }

    fn decode_simple(&mut self, additional: u8) -> Result<CanonicalValue, CanonicalError> {
        match additional {
            20 => Ok(CanonicalValue::Bool(false)),
            21 => Ok(CanonicalValue::Bool(true)),
            22 => Ok(CanonicalValue::Null),
            25 => {
                let bits = u16::from_be_bytes(self.read_array()?);
                Ok(CanonicalValue::Float(f16::from_bits(bits).to_f64()))
            }
            26 => {
                let bits = u32::from_be_bytes(self.read_array()?);
                Ok(CanonicalValue::Float(f64::from(f32::from_bits(bits))))
            }
            27 => {
                let bits = u64::from_be_bytes(self.read_array()?);
                Ok(CanonicalValue::Float(f64::from_bits(bits)))
            }
            31 => Err(CanonicalError::IndefiniteLength),
            _ => Err(CanonicalError::UnsupportedType {
                major: 7,
                additional,
            }),
        }
    }

    fn read_argument(&mut self, additional: u8) -> Result<u64, CanonicalError> {
        match additional {
            0..=23 => Ok(u64::from(additional)),
            24 => Ok(u64::from(self.read_byte()?)),
            25 => Ok(u64::from(u16::from_be_bytes(self.read_array()?))),
            26 => Ok(u64::from(u32::from_be_bytes(self.read_array()?))),
            27 => Ok(u64::from_be_bytes(self.read_array()?)),
            31 => Err(CanonicalError::IndefiniteLength),
            _ => Err(CanonicalError::UnsupportedType {
                major: 0,
                additional,
            }),
        }
    }

    fn read_scalar_length(&mut self, additional: u8) -> Result<usize, CanonicalError> {
        let length = self.read_argument(additional)?;
        if length > self.limits.max_scalar_bytes as u64 {
            return Err(CanonicalError::ScalarLimit {
                actual: length,
                maximum: self.limits.max_scalar_bytes,
            });
        }
        usize::try_from(length).map_err(|_| CanonicalError::LengthOverflow)
    }

    fn read_collection_length(&mut self, additional: u8) -> Result<usize, CanonicalError> {
        let length = self.read_argument(additional)?;
        if length > self.limits.max_collection_items as u64 {
            return Err(CanonicalError::CollectionLimit {
                actual: length,
                maximum: self.limits.max_collection_items,
            });
        }
        usize::try_from(length).map_err(|_| CanonicalError::LengthOverflow)
    }

    fn read_byte(&mut self) -> Result<u8, CanonicalError> {
        let value = self
            .bytes
            .get(self.offset)
            .copied()
            .ok_or(CanonicalError::Truncated {
                offset: self.offset,
            })?;
        self.offset += 1;
        Ok(value)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], CanonicalError> {
        let slice = self.read_slice(N)?;
        let mut output = [0_u8; N];
        output.copy_from_slice(slice);
        Ok(output)
    }

    fn read_slice(&mut self, length: usize) -> Result<&[u8], CanonicalError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(CanonicalError::LengthOverflow)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(CanonicalError::Truncated {
                offset: self.offset,
            })?;
        self.offset = end;
        Ok(value)
    }
}
