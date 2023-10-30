use camino::Utf8Path;
use sha2::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Sha512,
};
use snafu::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Snafu)]
pub enum Error {
    DigestFromExactIter,
    HexDecode { source: hex::FromHexError },
    Sha512EntryMissingDigest,
    Sha512EntryMissingFilename,
}

pub type Checksums<'a> = BTreeMap<&'a Utf8Path, Sha512Digest>;
pub type Sha512Digest = GenericArray<u8, <Sha512 as OutputSizeUser>::OutputSize>;

#[cfg_attr(feature = "tracing", tracing::instrument)]
pub fn parse_sha512_checksums(text: &str) -> Result<BTreeMap<&Utf8Path, Sha512Digest>, self::Error> {
    let mut entries = BTreeMap::new();
    for checksum in text.lines() {
        let mut parts = checksum.split_whitespace();
        let digest = {
            let digest = parts.next().context(Sha512EntryMissingDigestSnafu)?;
            let bytes = hex::decode(digest).context(HexDecodeSnafu)?;
            GenericArray::from_exact_iter(bytes).context(DigestFromExactIterSnafu)?
        };
        let filename = parts.next().context(Sha512EntryMissingFilenameSnafu)?;
        let filepath = Utf8Path::new(filename);
        entries.insert(filepath, digest);
    }
    Ok(entries)
}
