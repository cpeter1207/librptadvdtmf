use crate::{DtmfDigit, DtmfGenerator};

const DIGITS: [(DtmfDigit, f32, f32); 16] = [
    (DtmfDigit::One, 697.0, 1209.0),
    (DtmfDigit::Two, 697.0, 1336.0),
    (DtmfDigit::Three, 697.0, 1477.0),
    (DtmfDigit::A, 697.0, 1633.0),
    (DtmfDigit::Four, 770.0, 1209.0),
    (DtmfDigit::Five, 770.0, 1336.0),
    (DtmfDigit::Six, 770.0, 1477.0),
    (DtmfDigit::B, 770.0, 1633.0),
    (DtmfDigit::Seven, 852.0, 1209.0),
    (DtmfDigit::Eight, 852.0, 1336.0),
    (DtmfDigit::Nine, 852.0, 1477.0),
    (DtmfDigit::C, 852.0, 1633.0),
    (DtmfDigit::Star, 941.0, 1209.0),
    (DtmfDigit::Zero, 941.0, 1336.0),
    (DtmfDigit::Hash, 941.0, 1477.0),
    (DtmfDigit::D, 941.0, 1633.0),
];

fn collect(digit: DtmfDigit, partition: usize) -> Vec<f32> {
    let mut generator = DtmfGenerator::new(digit, 200, 50, -12).unwrap();
    assert_eq!(generator.render(&mut []).unwrap(), 0);
    let mut pcm = Vec::new();
    let mut block = vec![0.0; partition];
    loop {
        let written = generator.render(&mut block).unwrap();
        if written == 0 {
            assert_eq!(generator.render(&mut block).unwrap(), 0);
            break;
        }
        pcm.extend_from_slice(&block[..written]);
        assert!(pcm.len() <= 12000, "finite tone did not end");
    }
    pcm
}

fn amplitude(samples: &[f32], frequency: f32) -> f64 {
    let phase = f64::from(frequency) * std::f64::consts::TAU / 48000.0;
    let (real, imaginary) = samples.iter().enumerate().fold((0.0, 0.0), |sum, (i, x)| {
        let angle = i as f64 * phase;
        (
            sum.0 + f64::from(*x) * angle.cos(),
            sum.1 + f64::from(*x) * angle.sin(),
        )
    });
    2.0 * real.hypot(imaginary) / samples.len() as f64
}

#[test]
fn generator_all_symbols_match_frequency_level_and_duration() {
    for (digit, row, column) in DIGITS {
        let pcm = collect(digit, 12000);
        assert_eq!(pcm.len(), 12000);
        assert!(pcm[9600..].iter().all(|sample| *sample == 0.0));
        assert!(pcm.iter().all(|sample| sample.abs() <= 0.251189));
        for frequency in [697.0, 770.0, 852.0, 941.0, 1209.0, 1336.0, 1477.0, 1633.0] {
            let value = amplitude(&pcm[..9600], frequency);
            if frequency == row || frequency == column {
                assert!(
                    (0.119..0.132).contains(&value),
                    "{digit:?}: {frequency} Hz = {value}"
                );
            } else {
                assert!(value < 0.012, "unexpected {frequency} Hz = {value}");
            }
        }
    }
}

#[test]
fn generator_partitioning_matches_one_render() {
    for (digit, _, _) in DIGITS {
        let complete = collect(digit, 12000);
        for partition in [1, 137, 960, 4096] {
            assert_eq!(collect(digit, partition), complete);
        }
        assert_eq!(collect(digit, 12000), complete);
    }
}

#[test]
fn generator_validates_bounds_and_renders_extreme_durations() {
    for (duration, gap, level) in [
        (0, 50, -12),
        (1001, 50, -12),
        (200, 0, -12),
        (200, 1001, -12),
        (200, 50, -61),
        (200, 50, 1),
    ] {
        assert!(DtmfGenerator::new(DtmfDigit::One, duration, gap, level).is_err());
    }
    for (duration, gap, level, expected) in [(1, 1, -60, 96), (1000, 1000, 0, 96000)] {
        let mut generator = DtmfGenerator::new(DtmfDigit::One, duration, gap, level).unwrap();
        let mut pcm = vec![2.0_f32; expected + 1];
        assert_eq!(generator.render(&mut pcm).unwrap(), expected);
        assert!(pcm[..expected].iter().all(|sample| sample.abs() <= 1.0));
        assert!(
            pcm[duration as usize * 48..expected]
                .iter()
                .all(|sample| *sample == 0.0)
        );
        assert_eq!(pcm[expected], 2.0);
    }
}
