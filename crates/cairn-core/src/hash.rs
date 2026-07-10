use std::fmt;
use std::str::FromStr;

/// Exact 256-bit BLAKE3 digest.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Hash32([u8; 32]);

impl Hash32 {
    /// Compute the digest of the supplied bytes on the CPU.
    #[must_use]
    pub fn digest(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    /// Construct a digest from already validated bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the fixed-width digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Render lowercase hexadecimal without an algorithm prefix.
    #[must_use]
    pub fn to_hex(self) -> String {
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            let _format_cannot_fail = write!(output, "{byte:02x}");
        }
        output
    }
}

impl fmt::Debug for Hash32 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Hash32({self})")
    }
}

impl fmt::Display for Hash32 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "blake3:{}", self.to_hex())
    }
}

impl FromStr for Hash32 {
    type Err = HashParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let hex = value.strip_prefix("blake3:").unwrap_or(value);
        if hex.len() != 64 {
            return Err(HashParseError::WrongLength { actual: hex.len() });
        }
        let mut bytes = [0_u8; 32];
        for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
            let high =
                hex_digit(chunk[0]).ok_or(HashParseError::InvalidHex { index: index * 2 })?;
            let low = hex_digit(chunk[1]).ok_or(HashParseError::InvalidHex {
                index: index * 2 + 1,
            })?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

/// Precise digest text decoding failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HashParseError {
    WrongLength { actual: usize },
    InvalidHex { index: usize },
}

impl fmt::Display for HashParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongLength { actual } => {
                write!(formatter, "digest has {actual} hex bytes; expected 64")
            }
            Self::InvalidHex { index } => {
                write!(formatter, "invalid lowercase hex at byte {index}")
            }
        }
    }
}

impl std::error::Error for HashParseError {}

/// Semantic identity of a typed, versioned artifact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArtifactId(Hash32);

impl ArtifactId {
    /// Wrap a reconstructed semantic digest.
    #[must_use]
    pub const fn from_hash(hash: Hash32) -> Self {
        Self(hash)
    }

    /// Access the underlying BLAKE3 value.
    #[must_use]
    pub const fn hash(self) -> Hash32 {
        self.0
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for ArtifactId {
    type Err = HashParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

/// Content identity of an untyped immutable CAS blob.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BlobId(Hash32);

impl BlobId {
    #[must_use]
    pub const fn from_hash(hash: Hash32) -> Self {
        Self(hash)
    }

    #[must_use]
    pub const fn hash(self) -> Hash32 {
        self.0
    }
}

impl fmt::Display for BlobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for BlobId {
    type Err = HashParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}
