// SPDX-License-Identifier: Apache-2.0
//
// `HdDecoder` on a plain core stream (no extension substream): the frame is
// the core alone, the lossless taps stay empty and the integer core taps carry
// the PCM that the float samples were converted from. The in-tree stereo tone
// fixture is the input, so the test needs no external material.

use dca::{Extractor, HdDecoder, HdFrame};

const TONE: &[u8] = include_bytes!("../../harletty/tests/fixtures/dts_core_tone_10f.dts");

/// Core access units of the fixture.
fn access_units() -> Vec<Vec<u8>> {
    let mut extractor = Extractor::default();
    extractor.push_bytes(TONE);
    let mut units = Vec::new();
    while let Some(frame) = extractor.next_frame().expect("extract") {
        units.push(frame.as_bytes().to_vec());
    }
    units
}

#[test]
fn a_core_only_frame_is_decoded_and_read_through_the_core_taps() {
    let units = access_units();
    assert_eq!(units.len(), 10);
    let mut decoder = HdDecoder::new();
    let mut frame = HdFrame::default();
    let mut loud = false;
    for unit in &units {
        decoder
            .decode_into(unit, &[], &mut frame)
            .expect("a core without an extension substream decodes");
        assert!(!frame.lossless);
        assert!(!frame.x_present && frame.x_payload.is_empty() && frame.x_samples.is_empty());
        assert_eq!(decoder.lossless_samples().count(), 0);
        assert_eq!(decoder.lossless_x_samples().count(), 0);

        let taps: Vec<_> = decoder.core_samples().collect();
        assert_eq!(taps.len(), 2, "the fixture is stereo");
        for (speaker, ints) in taps {
            let floats = frame.samples[speaker].as_ref().expect("converted channel");
            assert_eq!(floats.len(), ints.len());
            for (f, &i) in floats.iter().zip(ints) {
                assert_eq!(*f, i as f32 / 8_388_608.0);
                loud |= i != 0;
            }
        }
    }
    assert!(loud, "the tone is not silent");
}

#[test]
fn turning_the_float_output_off_skips_the_conversion_and_keeps_the_taps() {
    let units = access_units();
    let mut with_floats = HdDecoder::new();
    let mut without = HdDecoder::new();
    without.set_float_output(false);
    let mut a = HdFrame::default();
    let mut b = HdFrame::default();
    for unit in &units {
        with_floats.decode_into(unit, &[], &mut a).expect("decode");
        without.decode_into(unit, &[], &mut b).expect("decode");

        assert!(a.samples.iter().any(Option::is_some));
        assert!(b.samples.is_empty() && b.x_samples.is_empty());
        assert_eq!(
            (a.sample_rate, a.output_mask, a.coded_mask),
            (b.sample_rate, b.output_mask, b.coded_mask),
            "the rest of the frame is filled as usual"
        );
        assert!(
            with_floats.core_samples().eq(without.core_samples()),
            "the integer taps do not depend on the conversion"
        );
    }
    // Turned back on, the conversion resumes.
    without.set_float_output(true);
    without.decode_into(&units[0], &[], &mut b).expect("decode");
    assert!(b.samples.iter().any(Option::is_some));
}
