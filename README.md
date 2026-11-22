# matroska-demuxer, featherbit fork

This is a fork of `matroska-demuxer` 0.8.1 by Nils Hasenbanck, kept for featherbittorrent. It is not affiliated with or endorsed by the upstream project. Report issues with the fork here. All credit for the original work goes to the upstream author.

## Use it

```toml
[patch.crates-io]
matroska-demuxer = { git = "https://github.com/featherbitplatform/matroska-demuxer", tag = "v0.8.1+featherbit.1" }
```

Each fork release is tagged `v<upstream version>+featherbit.<n>`, and consumers pin the newest tag.

## Origin

`upstream` is upstream `master`, unchanged. `main` starts at tag `v0.8.1` and adds the changes below as separate commits.

## Changes from 0.8.1

1. `src/lib.rs`, `MatroskaFile::seek_broad_phase` and `MatroskaFile::open`: seek to the cue's own cluster when the cue has a relative position.
   - Before: the cue's `CueRelativePosition` was added to the data offset of the first cluster of the segment (`cluster_start`), so every cue but the first landed in the wrong place for files that write relative positions (ffmpeg, mkvmerge).
   - After: the cluster at the cue's own `CueClusterPosition` is read (offset and timestamp), and the relative position is added to that.
   - Cue values come from the file, so the additions are overflow-safe. In `open`, `cluster_position += segment_data_offset` became `checked_add`, and a cue point whose sum overflows is dropped. In `seek_broad_phase`, an overflowing `cluster_data_offset + relative_position` falls back to the cue's cluster start and the narrow phase walks forward.
   - A cue that does not point at a readable cluster (the `Cues` element, the `SeekHead`, the middle of a cluster) is ignored and `seek` falls back to the linear search from the first cluster. A relative position at or past the end of a cluster of known size uses the cluster start instead.
   - Regression tests: `tests/seek_relative_position.rs` (builder in `tests/common/mod.rs`).
2. `src/lib.rs` (`seek_broad_phase`, `seek_narrow_phase`) and `src/block.rs` (`parse_timestamp`): checked arithmetic on values read from the file.
   - Before: `offset + size` for a cluster and for a SimpleBlock of unknown size (`0xFF`), and `cluster_timestamp + block timestamp`, panicked in debug builds and wrapped in release builds.
   - After: `checked_add`; overflow returns `DemuxError::InvalidEbmlDataSize` (cluster) or `DemuxError::InvalidBlockSize` (block offset and timestamp).
   - Regression tests: `tests/checked_arithmetic.rs`.
3. `Cargo.toml`: version `0.8.1+featherbit.1`, `repository` and `homepage` point at this fork, `publish = false`. The package name stays `matroska-demuxer` so it can replace the crates.io crate through `[patch.crates-io]`. This README section was added.

## Releasing

After a change, tag `main` with an annotated tag of the form `v<upstream version>+featherbit.<n>`.

## Version scheme

`<upstream version>+featherbit.<n>`. `n` counts this fork's revisions on one upstream release and restarts at 1 on a new upstream release.

## Updating from upstream

Fetch upstream, move the `upstream` branch to its new master, rebase or cherry-pick the fork's commits onto the new release tag, and reset `n` to 1.

---

# matroska-demuxer

[![Latest version](https://img.shields.io/crates/v/matroska-demuxer.svg)](https://crates.io/crates/matroska-demuxer)
[![Documentation](https://docs.rs/matroska-demuxer/badge.svg)](https://docs.rs/matroska-demuxer)
![ZLIB](https://img.shields.io/badge/license-zlib-blue.svg)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

A demuxer that can demux Matroska and WebM container files.

For simplicity only the elements supported by both Matroska and WebM are supported.

## Integration test

To run the integration test you need to
download [the Matroska test suite](https://sourceforge.net/projects/matroska/files/test_files/matroska_test_w1_1.zip/download)
video files and extract them into the `tests/data` folder (test1.mkv to test8.mkv).

## License

Licensed under MIT or Apache-2.0 or ZLIB.
