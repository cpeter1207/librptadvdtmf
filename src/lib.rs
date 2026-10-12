//! Portable DTMF detection state behind one versioned descriptor.

#![deny(warnings)]
#![cfg_attr(coverage, feature(coverage_attribute))]

use std::{
    ffi::{c_char, c_void},
    mem::size_of,
};

pub mod dtmf;
pub use dtmf::{DtmfDetector, DtmfDigit};

const ABI_VERSION: u32 = 1;
const CAPABILITY_NAME: &[u8] = b"rptadv.dtmf\0";

/// Opaque detector state.
pub struct DtmfHandle(DtmfDetector);

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
};

/// Return the immutable ABI 1 function table.
#[unsafe(no_mangle)]
pub extern "C" fn rptadv_dtmf_descriptor() -> *const DtmfDescriptor {
    &DESCRIPTOR
}

unsafe extern "C" fn create(muting: u32, out: *mut *mut DtmfHandle) -> i32 {
    let Some(out) = (unsafe { out.as_mut() }) else {
        return -1;
    };
    *out = Box::into_raw(Box::new(DtmfHandle(DtmfDetector::new(muting != 0))));
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
    state.0.process(receiving != 0, samples, |digit| {
        if let Some(callback) = emit {
            callback(context, ascii(digit));
        }
    });
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
