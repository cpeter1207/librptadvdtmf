//! Portable DTMF detection and finite generation behind one versioned descriptor.

#![deny(warnings)]
#![cfg_attr(coverage, feature(coverage_attribute))]

use std::{
    ffi::{c_char, c_void},
    mem::size_of,
};

pub mod dtmf;
pub use dtmf::{DtmfDetector, DtmfDigit};
mod generator;
mod tone_client;
pub use generator::{DtmfGenerator, GeneratorError};

const ABI_VERSION: u32 = 1;
const CAPABILITY_NAME: &[u8] = b"rptadv.dtmf\0";

/// Opaque detector state.
pub struct DtmfHandle(DtmfDetector);

/// Opaque finite generator; prepare and release it off audio.
pub struct DtmfGeneratorHandle(DtmfGenerator);

/// ABI 1 DTMF function table.
#[repr(C)]
pub struct DtmfDescriptor {
    struct_size: u32,
    abi_version: u32,
    capability_name: *const c_char,
    create: unsafe extern "C" fn(u32, *mut *mut DtmfHandle) -> i32,
    destroy: unsafe extern "C" fn(*mut DtmfHandle),
    set_muting: unsafe extern "C" fn(*mut DtmfHandle, u32),
    suppressing: unsafe extern "C" fn(*const DtmfHandle) -> u32,
    process: unsafe extern "C" fn(
        *mut DtmfHandle,
        u32,
        *mut f32,
        u32,
        Option<extern "C" fn(*mut c_void, u8)>,
        *mut c_void,
    ),
    generator_create: unsafe extern "C" fn(u8, u32, u32, i32, *mut *mut DtmfGeneratorHandle) -> i32,
    generator_render:
        unsafe extern "C" fn(*mut DtmfGeneratorHandle, *mut f32, usize, *mut usize) -> i32,
    generator_destroy: unsafe extern "C" fn(*mut DtmfGeneratorHandle),
    create_at_rate: unsafe extern "C" fn(u32, u32, *mut *mut DtmfHandle) -> i32,
    process_spans: unsafe extern "C" fn(
        *mut DtmfHandle,
        u32,
        *mut f32,
        u32,
        Option<extern "C" fn(*mut c_void, u8)>,
        *mut c_void,
        Option<extern "C" fn(*mut c_void, i64, i64)>,
        *mut c_void,
    ),
}

// SAFETY: the descriptor and every address it references are immutable.
unsafe impl Sync for DtmfDescriptor {}

static DESCRIPTOR: DtmfDescriptor = DtmfDescriptor {
    struct_size: size_of::<DtmfDescriptor>() as u32,
    abi_version: ABI_VERSION,
    capability_name: CAPABILITY_NAME.as_ptr().cast::<c_char>(),
    create,
    destroy,
    set_muting,
    suppressing,
    process,
    generator_create,
    generator_render,
    generator_destroy,
    create_at_rate,
    process_spans,
};

/// Return the immutable ABI 1 function table.
#[unsafe(no_mangle)]
pub extern "C" fn rptadv_dtmf_descriptor() -> *const DtmfDescriptor {
    &DESCRIPTOR
}

unsafe extern "C" fn create(muting: u32, out: *mut *mut DtmfHandle) -> i32 {
    unsafe { create_at_rate(muting, 48_000, out) }
}

unsafe extern "C" fn create_at_rate(muting: u32, rate: u32, out: *mut *mut DtmfHandle) -> i32 {
    let Some(out) = (unsafe { out.as_mut() }) else {
        return -1;
    };
    *out = std::ptr::null_mut();
    let Some(detector) = DtmfDetector::at_rate(muting != 0, rate) else {
        return -1;
    };
    *out = Box::into_raw(Box::new(DtmfHandle(detector)));
    0
}

unsafe extern "C" fn destroy(handle: *mut DtmfHandle) {
    if !handle.is_null() {
        // SAFETY: ownership returns exactly once after audio callbacks stop.
        drop(unsafe { Box::from_raw(handle) });
    }
}

unsafe extern "C" fn set_muting(handle: *mut DtmfHandle, enabled: u32) {
    // SAFETY: the caller holds the handle exclusively.
    if let Some(state) = unsafe { handle.as_mut() } {
        state.0.set_muting(enabled != 0);
    }
}

unsafe extern "C" fn suppressing(handle: *const DtmfHandle) -> u32 {
    // SAFETY: the caller holds the handle for this call.
    u32::from(unsafe { handle.as_ref() }.is_some_and(|state| state.0.suppressing()))
}

unsafe extern "C" fn process(
    handle: *mut DtmfHandle,
    receiving: u32,
    audio: *mut f32,
    count: u32,
    emit: Option<extern "C" fn(*mut c_void, u8)>,
    context: *mut c_void,
) {
    unsafe {
        process_spans(
            handle,
            receiving,
            audio,
            count,
            emit,
            context,
            None,
            std::ptr::null_mut(),
        );
    }
}

unsafe extern "C" fn process_spans(
    handle: *mut DtmfHandle,
    receiving: u32,
    audio: *mut f32,
    count: u32,
    emit: Option<extern "C" fn(*mut c_void, u8)>,
    context: *mut c_void,
    mute: Option<extern "C" fn(*mut c_void, i64, i64)>,
    mute_context: *mut c_void,
) {
    // SAFETY: the caller holds this handle exclusively and supplies a writable
    // count-sized span. A zero-length carrier-loss call may use a null pointer.
    let Some(state) = (unsafe { handle.as_mut() }) else {
        return;
    };
    if count != 0 && audio.is_null() {
        return;
    }
    let samples = if count == 0 {
        &mut []
    } else {
        // SAFETY: the non-null pointer names count writable f32 samples.
        unsafe { std::slice::from_raw_parts_mut(audio, count as usize) }
    };
    state.0.process_spans(
        receiving != 0,
        samples,
        |digit| {
            if let Some(callback) = emit {
                callback(context, ascii(digit));
            }
        },
        |start, end| {
            if let Some(callback) = mute {
                callback(mute_context, start, end);
            }
        },
    );
}

fn ascii(digit: DtmfDigit) -> u8 {
    match digit {
        DtmfDigit::One => b'1',
        DtmfDigit::Two => b'2',
        DtmfDigit::Three => b'3',
        DtmfDigit::Four => b'4',
        DtmfDigit::Five => b'5',
        DtmfDigit::Six => b'6',
        DtmfDigit::Seven => b'7',
        DtmfDigit::Eight => b'8',
        DtmfDigit::Nine => b'9',
        DtmfDigit::Zero => b'0',
        DtmfDigit::Star => b'*',
        DtmfDigit::Hash => b'#',
        DtmfDigit::A => b'A',
        DtmfDigit::B => b'B',
        DtmfDigit::C => b'C',
        DtmfDigit::D => b'D',
    }
}

unsafe extern "C" fn generator_create(
    digit: u8,
    duration_ms: u32,
    gap_ms: u32,
    level_dbfs: i32,
    out: *mut *mut DtmfGeneratorHandle,
) -> i32 {
    // SAFETY: caller supplies null or a writable output pointer.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return -1;
    };
    *out = std::ptr::null_mut();
    use DtmfDigit::*;
    let digit = [
        One, Two, Three, A, Four, Five, Six, B, Seven, Eight, Nine, C, Star, Zero, Hash, D,
    ]
    .into_iter()
    .find(|value| ascii(*value) == digit);
    let generator = digit
        .zip(i8::try_from(level_dbfs).ok())
        .and_then(|(digit, level)| DtmfGenerator::new(digit, duration_ms, gap_ms, level).ok());
    let Some(generator) = generator else {
        return -1;
    };
    *out = Box::into_raw(Box::new(DtmfGeneratorHandle(generator)));
    0
}

unsafe extern "C" fn generator_render(
    handle: *mut DtmfGeneratorHandle,
    output: *mut f32,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    // SAFETY: caller supplies null or a writable count and exclusively owned handle.
    let Some(written) = (unsafe { written.as_mut() }) else {
        return -1;
    };
    *written = 0;
    let Some(generator) = (unsafe { handle.as_mut() }) else {
        return -1;
    };
    let samples = if capacity == 0 {
        &mut []
    } else if output.is_null() {
        return -1;
    } else {
        // SAFETY: non-null output is writable for capacity samples by the ABI contract.
        unsafe { std::slice::from_raw_parts_mut(output, capacity) }
    };
    match generator.0.render(samples) {
        Ok(count) => {
            *written = count;
            0
        }
        Err(_) => -1,
    }
}

unsafe extern "C" fn generator_destroy(handle: *mut DtmfGeneratorHandle) {
    if !handle.is_null() {
        // SAFETY: ownership returns once, after the last render operation completes.
        drop(unsafe { Box::from_raw(handle) });
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn descriptor_create_uses_u32_flags_and_status_out_handle() {
        let api = unsafe { &*rptadv_dtmf_descriptor() };
        let create: unsafe extern "C" fn(u32, *mut *mut DtmfHandle) -> i32 = api.create;
        let mut detector = std::ptr::null_mut();
        assert_eq!(unsafe { create(2, &mut detector) }, 0);
        assert!(!detector.is_null());
        assert_eq!(unsafe { create(0, std::ptr::null_mut()) }, -1);
        unsafe { (api.destroy)(detector) };
    }

    #[test]
    fn descriptor_exposes_borrowed_ascii_events_and_muted_pcm() {
        let api = unsafe { rptadv_dtmf_descriptor().as_ref() }.expect("descriptor");
        assert_eq!(api.abi_version, 1);
        assert_eq!(api.struct_size as usize, size_of::<DtmfDescriptor>());
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(api.capability_name) }.to_bytes(),
            b"rptadv.dtmf"
        );
        let mut detector = std::ptr::null_mut();
        assert_eq!(unsafe { (api.create)(2, &mut detector) }, 0);
        assert!(!detector.is_null());
        let mut digits = Vec::<u8>::new();
        extern "C" fn capture(context: *mut c_void, digit: u8) {
            unsafe { &mut *context.cast::<Vec<u8>>() }.push(digit);
        }
        let mut tone = Vec::new();
        for sample in 0..(3 * 612) {
            let time = sample as f32 / 48_000.0;
            tone.push(
                0.031
                    * ((std::f32::consts::TAU * 697.0 * time).sin()
                        + (std::f32::consts::TAU * 1209.0 * time).sin()),
            );
        }
        unsafe {
            (api.process)(
                detector,
                2,
                tone.as_mut_ptr(),
                tone.len() as u32,
                Some(capture),
                (&mut digits as *mut Vec<u8>).cast(),
            );
            assert_eq!((api.suppressing)(detector), 1);
            assert!(tone[2 * 612..].iter().all(|sample| *sample == 0.0));
            (api.set_muting)(detector, 0);
            assert_eq!((api.suppressing)(detector), 0);
            (api.set_muting)(detector, 2);
            assert_eq!((api.suppressing)(detector), 1);
            (api.process)(
                detector,
                0,
                std::ptr::null_mut(),
                0,
                Some(capture),
                (&mut digits as *mut Vec<u8>).cast(),
            );
            assert_eq!(digits, [b'1']);
            (api.destroy)(detector);
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod abi_tests;

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod generator_tests;
