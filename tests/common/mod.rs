//! In-memory Matroska builder shared by the integration tests.
#![allow(dead_code)]

/// Encodes an element size as a minimal variable-size integer.
pub fn vint(value: u64) -> Vec<u8> {
    for len in 1..=8u32 {
        // The all-ones pattern is reserved for "unknown size".
        if value < (1u64 << (7 * len)) - 1 {
            let marked = value | (1u64 << (7 * len));
            return marked.to_be_bytes()[(8 - len as usize)..].to_vec();
        }
    }
    panic!("size too large");
}

pub fn element(id: &[u8], body: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend(vint(body.len() as u64));
    out.extend_from_slice(body);
    out
}

/// Unsigned integer with the minimal number of bytes.
pub fn uint(id: &[u8], value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let skip = bytes.iter().take_while(|b| **b == 0).count().min(7);
    element(id, &bytes[skip..])
}

/// Unsigned integer with a fixed width of four bytes (keeps layouts stable between passes).
pub fn uint32(id: &[u8], value: u64) -> Vec<u8> {
    element(id, &(value as u32).to_be_bytes())
}

pub fn cat(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.concat()
}

pub const FRAME_OFFSETS: [u64; 3] = [0, 600, 900];

pub struct Cue {
    pub time: u64,
    pub cluster_position: u64,
    pub relative_position: u64,
}

pub fn build_cluster(timestamp: u64) -> Vec<u8> {
    let mut body = uint(&[0xE7], timestamp);
    for (i, offset) in FRAME_OFFSETS.iter().enumerate() {
        let flags = if i == 0 { 0x80 } else { 0x00 };
        let mut block = vec![0x81];
        block.extend((*offset as i16).to_be_bytes());
        block.push(flags);
        block.extend([0xAA, 0xBB, i as u8]);
        body.extend(element(&[0xA3], &block));
    }
    element(&[0x1F, 0x43, 0xB6, 0x75], &body)
}

/// Builds a file with one cluster per timestamp. `tweak` can alter the cues before they are
/// written.
pub fn build_file(timestamps: [u64; 3], tweak: impl Fn(&mut Vec<Cue>)) -> Vec<u8> {
    let header = element(
        &[0x1A, 0x45, 0xDF, 0xA3],
        &cat(&[
            element(&[0x42, 0x82], b"matroska"),
            uint(&[0x42, 0x87], 4),
            uint(&[0x42, 0x85], 2),
        ]),
    );

    let info = element(
        &[0x15, 0x49, 0xA9, 0x66],
        &cat(&[
            uint(&[0x2A, 0xD7, 0xB1], 1_000_000),
            element(&[0x4D, 0x80], b"test"),
            element(&[0x57, 0x41], b"test"),
        ]),
    );
    let video = element(&[0xE0], &cat(&[uint(&[0xB0], 64), uint(&[0xBA], 48)]));
    let track_entry = element(
        &[0xAE],
        &cat(&[
            uint(&[0xD7], 1),
            uint(&[0x73, 0xC5], 1),
            uint(&[0x83], 1),
            element(&[0x86], b"V_MPEG4/ISO/AVC"),
            element(&[0x63, 0xA2], &[0x01, 0x64, 0x00, 0x1F]),
            video,
        ]),
    );
    let tracks = element(&[0x16, 0x54, 0xAE, 0x6B], &track_entry);

    let seek_head = |info: u64, tracks: u64, cues: u64, cluster: u64| {
        let entry = |id: &[u8], pos: u64| {
            element(
                &[0x4D, 0xBB],
                &cat(&[element(&[0x53, 0xAB], id), uint32(&[0x53, 0xAC], pos)]),
            )
        };
        element(
            &[0x11, 0x4D, 0x9B, 0x74],
            &cat(&[
                entry(&[0x15, 0x49, 0xA9, 0x66], info),
                entry(&[0x16, 0x54, 0xAE, 0x6B], tracks),
                entry(&[0x1C, 0x53, 0xBB, 0x6B], cues),
                entry(&[0x1F, 0x43, 0xB6, 0x75], cluster),
            ]),
        )
    };
    let seek_head_len = seek_head(0, 0, 0, 0).len() as u64;

    // All positions are relative to the start of the segment data.
    let info_pos = seek_head_len;
    let tracks_pos = info_pos + info.len() as u64;
    let first_cluster_pos = tracks_pos + tracks.len() as u64;

    let clusters: Vec<Vec<u8>> = timestamps.iter().map(|t| build_cluster(*t)).collect();
    let mut cues = Vec::new();
    let mut pos = first_cluster_pos;
    for (time, cluster) in timestamps.iter().zip(&clusters) {
        // The first SimpleBlock follows the Timestamp element directly.
        let first_block = uint(&[0xE7], *time).len() as u64;
        cues.push(Cue {
            time: *time,
            cluster_position: pos,
            relative_position: first_block,
        });
        pos += cluster.len() as u64;
    }
    let cues_pos = pos;
    tweak(&mut cues);

    let cue_elements: Vec<Vec<u8>> = cues
        .iter()
        .map(|c| {
            element(
                &[0xBB],
                &cat(&[
                    uint(&[0xB3], c.time),
                    element(
                        &[0xB7],
                        &cat(&[
                            uint(&[0xF7], 1),
                            uint(&[0xF1], c.cluster_position),
                            uint(&[0xF0], c.relative_position),
                        ]),
                    ),
                ]),
            )
        })
        .collect();
    let cues_el = element(&[0x1C, 0x53, 0xBB, 0x6B], &cat(&cue_elements));

    let segment_body = cat(&[
        seek_head(info_pos, tracks_pos, cues_pos, first_cluster_pos),
        info,
        tracks,
        cat(&clusters),
        cues_el,
    ]);
    cat(&[header, element(&[0x18, 0x53, 0x80, 0x67], &segment_body)])
}
