//! Hostile in-memory files must not overflow-panic in debug builds.

mod common;

use std::io::Cursor;

use common::build_file;
use matroska_demuxer::{DemuxError, MatroskaFile};

#[test]
fn block_of_unknown_size_is_an_error() {
    let mut data = build_file([0, 1000, 2000], |_| {});
    // The first SimpleBlock (id A3, 7 byte body) gets the "unknown size" marker.
    let at = data
        .windows(2)
        .position(|w| w == [0xA3, 0x87])
        .expect("simple block");
    data[at + 1] = 0xFF;

    let mut file = MatroskaFile::open(Cursor::new(data)).expect("open");
    let result = file.seek(1);
    assert!(
        matches!(result, Err(DemuxError::InvalidBlockSize)),
        "{result:?}"
    );
}

#[test]
fn cluster_timestamp_near_u64_max_is_an_error() {
    let data = build_file([0, 1000, u64::MAX - 5], |_| {});

    let mut file = MatroskaFile::open(Cursor::new(data)).expect("open");
    let result = file.seek(u64::MAX - 1);
    assert!(
        matches!(result, Err(DemuxError::InvalidBlockSize)),
        "{result:?}"
    );
}
