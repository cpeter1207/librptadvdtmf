use super::*;

const FRAME: usize = 612;

extern "C" fn capture(context: *mut c_void, digit: u8) {
    // SAFETY: process borrows this live Vec for the duration of one call.
    unsafe { &mut *context.cast::<Vec<u8>>() }.push(digit);
}

fn tone(digit: DtmfDigit, first: usize) -> Vec<f32> {
    let (row, column) = digit.frequencies();
    (0..FRAME)
        .map(|offset| {
            let time = (first + offset) as f32 / 48_000.0;
            0.031
                * ((core::f32::consts::TAU * row * time).sin()
                    + (core::f32::consts::TAU * column * time).sin())
        })
        .collect()
}

fn fixture() -> Vec<f32> {
    let mut samples = Vec::new();
    for _ in 0..4 {
        samples.extend(tone(DtmfDigit::Five, samples.len()));
    }
    samples.resize(samples.len() + FRAME, 0.0);
    samples.extend(tone(DtmfDigit::Five, samples.len()));
    samples.resize(samples.len() + 3 * FRAME, 0.0);
    for _ in 0..2 {
        samples.extend(tone(DtmfDigit::Two, samples.len()));
    }
    samples
}

#[test]
fn shared_dtmf_matches_baseline_events_and_samples() {
    let api = unsafe { &*rptadv_dtmf_descriptor() };
    let input = fixture();
    for partition in [1, 37, FRAME, 960, input.len()] {
        let mut baseline = DtmfDetector::new(true);
        let mut baseline_audio = input.clone();
        let mut baseline_events = Vec::new();
        for block in baseline_audio.chunks_mut(partition) {
            baseline.process(true, block, |digit| baseline_events.push(ascii(digit)));
        }
        baseline.process(false, &mut [], |digit| baseline_events.push(ascii(digit)));

        let detector = (api.create)(true);
        let mut shared_audio = input.clone();
        let mut shared_events = Vec::new();
        let context = (&mut shared_events as *mut Vec<u8>).cast();
        unsafe {
            for block in shared_audio.chunks_mut(partition) {
                (api.process)(
                    detector,
                    true,
                    block.as_mut_ptr(),
                    block.len() as u32,
                    Some(capture),
                    context,
                );
            }
            (api.process)(
                detector,
                false,
                std::ptr::null_mut(),
                0,
                Some(capture),
                context,
            );
            assert!(!(api.suppressing)(detector));
            (api.destroy)(detector);
        }
        assert_eq!(baseline_events, [b'5', b'2']);
        assert_eq!(shared_events, baseline_events, "partition {partition}");
        assert_eq!(shared_audio, baseline_audio, "partition {partition}");
        assert!(
            shared_audio[2 * FRAME..4 * FRAME]
                .iter()
                .all(|sample| *sample == 0.0)
        );
        assert_eq!(shared_audio[..2 * FRAME], input[..2 * FRAME]);
    }
}

#[test]
fn descriptor_emits_every_standard_ascii_digit() {
    let api = unsafe { &*rptadv_dtmf_descriptor() };
    let symbols = [
        (DtmfDigit::One, b'1'),
        (DtmfDigit::Two, b'2'),
        (DtmfDigit::Three, b'3'),
        (DtmfDigit::A, b'A'),
        (DtmfDigit::Four, b'4'),
        (DtmfDigit::Five, b'5'),
        (DtmfDigit::Six, b'6'),
        (DtmfDigit::B, b'B'),
        (DtmfDigit::Seven, b'7'),
        (DtmfDigit::Eight, b'8'),
        (DtmfDigit::Nine, b'9'),
        (DtmfDigit::C, b'C'),
        (DtmfDigit::Star, b'*'),
        (DtmfDigit::Zero, b'0'),
        (DtmfDigit::Hash, b'#'),
        (DtmfDigit::D, b'D'),
    ];
    for (digit, expected) in symbols {
        let detector = (api.create)(false);
        let mut events = Vec::new();
        let context = (&mut events as *mut Vec<u8>).cast();
        unsafe {
            for block in 0..2 {
                let mut audio = tone(digit, block * FRAME);
                (api.process)(
                    detector,
                    true,
                    audio.as_mut_ptr(),
                    FRAME as u32,
                    Some(capture),
                    context,
                );
            }
            (api.process)(
                detector,
                false,
                std::ptr::null_mut(),
                0,
                Some(capture),
                context,
            );
            (api.destroy)(detector);
        }
        assert_eq!(events, [expected]);
    }
}

#[test]
fn descriptor_ignores_invalid_buffers_and_allows_no_callback() {
    let api = unsafe { &*rptadv_dtmf_descriptor() };
    let detector = (api.create)(true);
    let mut samples = [0.25_f32];
    unsafe {
        (api.process)(
            std::ptr::null_mut(),
            true,
            samples.as_mut_ptr(),
            1,
            None,
            std::ptr::null_mut(),
        );
        (api.process)(
            detector,
            true,
            std::ptr::null_mut(),
            1,
            None,
            std::ptr::null_mut(),
        );
        (api.process)(
            detector,
            true,
            std::ptr::null_mut(),
            0,
            None,
            std::ptr::null_mut(),
        );
        assert_eq!(samples, [0.25]);
        (api.set_muting)(detector, false);
        assert!(!(api.suppressing)(detector));
        (api.set_muting)(std::ptr::null_mut(), true);
        assert!(!(api.suppressing)(std::ptr::null()));
        let mut digit_audio = Vec::new();
        for _ in 0..2 {
            digit_audio.extend(tone(DtmfDigit::One, digit_audio.len()));
        }
        (api.process)(
            detector,
            true,
            digit_audio.as_mut_ptr(),
            digit_audio.len() as u32,
            None,
            std::ptr::null_mut(),
        );
        (api.process)(
            detector,
            false,
            std::ptr::null_mut(),
            0,
            None,
            std::ptr::null_mut(),
        );
        (api.destroy)(detector);
        (api.destroy)(std::ptr::null_mut());
    }
}
