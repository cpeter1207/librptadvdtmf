use super::*;

const FRAME: usize = 612;

extern "C" fn capture(context: *mut c_void, digit: u8) {
    // SAFETY: process borrows this live Vec for the duration of one call.
    unsafe { &mut *context.cast::<Vec<u8>>() }.push(digit);
}

fn create(api: &DtmfDescriptor, muting: u32) -> *mut DtmfHandle {
    let mut detector = std::ptr::null_mut();
    assert_eq!(unsafe { (api.create)(muting, &mut detector) }, 0);
    assert!(!detector.is_null());
    detector
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

fn twisted(first: usize) -> Vec<f32> {
    (0..FRAME)
        .map(|offset| {
            let phase = core::f32::consts::TAU * (first + offset) as f32 / 48_000.0;
            0.05 * (phase * 697.0).sin() + 0.8 * (phase * 1209.0).sin()
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
    for _ in 0..5 {
        samples.extend(twisted(samples.len()));
    }
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

        let detector = create(api, 1);
        let mut shared_audio = input.clone();
        let mut shared_events = Vec::new();
        let context = (&mut shared_events as *mut Vec<u8>).cast();
        unsafe {
            for block in shared_audio.chunks_mut(partition) {
                (api.process)(
                    detector,
                    1,
                    block.as_mut_ptr(),
                    block.len() as u32,
                    Some(capture),
                    context,
                );
            }
            (api.process)(detector, 0, std::ptr::null_mut(), 0, Some(capture), context);
            assert_eq!((api.suppressing)(detector), 0);
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
        assert_eq!(
            shared_audio[9 * FRAME..14 * FRAME],
            input[9 * FRAME..14 * FRAME]
        );
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
        let detector = create(api, 0);
        let mut events = Vec::new();
        let context = (&mut events as *mut Vec<u8>).cast();
        unsafe {
            for block in 0..2 {
                let mut audio = tone(digit, block * FRAME);
                (api.process)(
                    detector,
                    1,
                    audio.as_mut_ptr(),
                    FRAME as u32,
                    Some(capture),
                    context,
                );
            }
            (api.process)(detector, 0, std::ptr::null_mut(), 0, Some(capture), context);
            (api.destroy)(detector);
        }
        assert_eq!(events, [expected]);
    }
}

#[test]
fn descriptor_ignores_invalid_buffers_and_allows_no_callback() {
    let api = unsafe { &*rptadv_dtmf_descriptor() };
    let detector = create(api, 1);
    let mut samples = [0.25_f32];
    unsafe {
        (api.process)(
            std::ptr::null_mut(),
            1,
            samples.as_mut_ptr(),
            1,
            None,
            std::ptr::null_mut(),
        );
        (api.process)(
            detector,
            1,
            std::ptr::null_mut(),
            1,
            None,
            std::ptr::null_mut(),
        );
        (api.process)(
            detector,
            1,
            std::ptr::null_mut(),
            0,
            None,
            std::ptr::null_mut(),
        );
        assert_eq!(samples, [0.25]);
        (api.set_muting)(detector, 0);
        assert_eq!((api.suppressing)(detector), 0);
        (api.set_muting)(std::ptr::null_mut(), 2);
        assert_eq!((api.suppressing)(std::ptr::null()), 0);
        let mut digit_audio = Vec::new();
        for _ in 0..2 {
            digit_audio.extend(tone(DtmfDigit::One, digit_audio.len()));
        }
        (api.process)(
            detector,
            2,
            digit_audio.as_mut_ptr(),
            digit_audio.len() as u32,
            None,
            std::ptr::null_mut(),
        );
        (api.process)(
            detector,
            0,
            std::ptr::null_mut(),
            0,
            None,
            std::ptr::null_mut(),
        );
        (api.destroy)(detector);
        (api.destroy)(std::ptr::null_mut());
    }
}
