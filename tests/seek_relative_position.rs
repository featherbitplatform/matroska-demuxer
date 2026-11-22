//! Regression tests for seeking with cues that carry `CueRelativePosition`.

mod common;

use std::io::Cursor;

use common::{build_file, Cue};
use matroska_demuxer::{Frame, MatroskaFile};

const TIMESTAMPS: [u64; 3] = [0, 1000, 2000];
const HUGE: u64 = u64::MAX - 10;

fn open(tweak: impl Fn(&mut Vec<Cue>)) -> MatroskaFile<Cursor<Vec<u8>>> {
    MatroskaFile::open(Cursor::new(build_file(TIMESTAMPS, tweak))).expect("open")
}

/// Seeks and returns the timestamp of the next frame.
fn frame_at(file: &mut MatroskaFile<Cursor<Vec<u8>>>, at: u64) -> u64 {
    let mut frame = Frame::default();
    file.seek(at).expect("seek");
    assert!(file.next_frame(&mut frame).expect("next_frame"));
    frame.timestamp
}

#[test]
fn seek_lands_in_the_cues_own_cluster() {
    let mut file = open(|_| {});

    // The first frame at or after 1500 is the second block of the cluster at 1000.
    assert_eq!(frame_at(&mut file, 1500), 1600);
    assert_eq!(frame_at(&mut file, 1000), 1000);
    assert_eq!(frame_at(&mut file, 2000), 2000);
}

#[test]
fn hostile_cue_positions_do_not_panic() {
    let variants: [fn(&mut Vec<Cue>); 2] = [
        |cues| cues[1].relative_position = HUGE,
        |cues| cues[1].cluster_position = HUGE,
    ];
    for tweak in variants {
        let mut file = open(tweak);
        let mut frame = Frame::default();
        for at in [0, 500, 1000, 1500, 2000, 5000] {
            if file.seek(at).is_ok() {
                let _ = file.next_frame(&mut frame);
            }
        }
    }
}

#[test]
fn overflowing_relative_position_falls_back_to_the_cluster_start() {
    let mut file = open(|cues| cues[1].relative_position = HUGE);
    assert_eq!(frame_at(&mut file, 1000), 1000);
}

#[test]
fn cue_with_overflowing_cluster_position_is_dropped() {
    let mut file = open(|cues| cues[1].cluster_position = HUGE);
    assert_eq!(frame_at(&mut file, 0), 0);
    assert_eq!(frame_at(&mut file, 2000), 2000);
}

#[test]
fn cue_pointing_at_a_non_cluster_element_falls_back_to_linear_search() {
    // Position 0 is the SeekHead element, not a Cluster.
    let mut file = open(|cues| cues[1].cluster_position = 0);
    assert_eq!(frame_at(&mut file, 1500), 1600);
}

#[test]
fn relative_position_past_the_cluster_uses_the_cluster_start() {
    let mut file = open(|cues| cues[1].relative_position = 10_000);
    assert_eq!(frame_at(&mut file, 1500), 1600);
}
