//! Narrow dynamic client for finite sequences in the released tone descriptor.

use crate::GeneratorError;
use std::{
    ffi::{CStr, c_char},
    mem::size_of,
    ptr::NonNull,
};

#[repr(C)]
struct Prefix {
    size: u32,
    abi: u32,
    capability: *const c_char,
}

type Create = unsafe extern "C" fn(*const u8, usize, i8, *mut *mut ()) -> i32;
type Render = unsafe extern "C" fn(*mut (), *mut f32, usize, *mut usize) -> i32;
type Destroy = unsafe extern "C" fn(*mut ());

/// Only the ABI prefix used by finite sequences; oscillator entries are not read.
#[repr(C)]
struct Descriptor {
    prefix: Prefix,
    create: Option<Create>,
    samples: Option<unsafe extern "C" fn(*const (), *mut usize) -> i32>,
    render: Option<Render>,
    destroy: Option<Destroy>,
}

#[link(name = "rptadvtone")]
unsafe extern "C" {
    fn rptadv_tone_descriptor() -> *const Prefix;
}

/// Single-owner provider handle; callback addresses remain valid for the DSO lifetime.
pub(crate) struct Sequence {
    handle: NonNull<()>,
    render: Render,
    destroy: Destroy,
}

// SAFETY: the unique handle may move between threads but is never used concurrently.
unsafe impl Send for Sequence {}

impl Sequence {
    pub(crate) fn new(text: &str, level: i8) -> Result<Self, GeneratorError> {
        // SAFETY: the linked provider publishes an immutable descriptor for process lifetime.
        unsafe { Self::from_descriptor(rptadv_tone_descriptor(), text, level) }
    }

    unsafe fn from_descriptor(
        pointer: *const Prefix,
        text: &str,
        level: i8,
    ) -> Result<Self, GeneratorError> {
        // SAFETY: caller provides null or a readable provider prefix.
        let prefix = unsafe { pointer.as_ref() }.ok_or(GeneratorError)?;
        if (prefix.size as usize) < size_of::<Descriptor>()
            || prefix.abi != 1
            || prefix.capability.is_null()
            || unsafe { CStr::from_ptr(prefix.capability) } != c"rptadv.tone"
        {
            return Err(GeneratorError);
        }
        // SAFETY: the validated prefix covers all sequence callbacks.
        let api = unsafe { &*pointer.cast::<Descriptor>() };
        let (Some(create), Some(render), Some(destroy)) = (api.create, api.render, api.destroy)
        else {
            return Err(GeneratorError);
        };
        let mut handle = std::ptr::null_mut();
        // SAFETY: creation borrows the valid text and writes an owned opaque handle.
        if unsafe { create(text.as_ptr(), text.len(), level, &mut handle) } != 0 {
            return Err(GeneratorError);
        }
        Ok(Self {
            handle: NonNull::new(handle).ok_or(GeneratorError)?,
            render,
            destroy,
        })
    }

    pub(crate) fn render(&mut self, output: &mut [f32]) -> Result<usize, GeneratorError> {
        let mut written = 0;
        // SAFETY: unique live handle, writable output span and count.
        let result = unsafe {
            (self.render)(
                self.handle.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
                &mut written,
            )
        };
        if result != 0 || written > output.len() {
            return Err(GeneratorError);
        }
        Ok(written)
    }
}

impl Drop for Sequence {
    fn drop(&mut self) {
        // SAFETY: the sole owner returns its handle after its last render.
        unsafe { (self.destroy)(self.handle.as_ptr()) };
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn generator_tone_descriptor_rejects_short_wrong_and_missing_apis() {
        unsafe {
            assert!(Sequence::from_descriptor(std::ptr::null(), "1/1", 0).is_err());
            for (size, abi, capability) in [
                (0, 1, c"rptadv.tone".as_ptr()),
                (size_of::<Descriptor>() as u32, 2, c"rptadv.tone".as_ptr()),
                (size_of::<Descriptor>() as u32, 1, std::ptr::null()),
                (size_of::<Descriptor>() as u32, 1, c"wrong".as_ptr()),
            ] {
                let prefix = Prefix {
                    size,
                    abi,
                    capability,
                };
                assert!(Sequence::from_descriptor(&prefix, "1/1", 0).is_err());
            }
            for missing in 0..3 {
                let mut api = rptadv_tone_descriptor().cast::<Descriptor>().read();
                match missing {
                    0 => api.create = None,
                    1 => api.render = None,
                    _ => api.destroy = None,
                }
                assert!(Sequence::from_descriptor(&api.prefix, "1/1", 0).is_err());
            }
        }
        assert!(Sequence::new("invalid", 0).is_err());
    }

    unsafe extern "C" fn empty_create(_: *const u8, _: usize, _: i8, _: *mut *mut ()) -> i32 {
        0
    }
    unsafe extern "C" fn failed_render(_: *mut (), _: *mut f32, _: usize, _: *mut usize) -> i32 {
        -1
    }
    unsafe extern "C" fn oversized_render(
        _: *mut (),
        _: *mut f32,
        capacity: usize,
        written: *mut usize,
    ) -> i32 {
        unsafe {
            *written = capacity + 1;
        }
        0
    }

    pub(crate) fn failing_sequences() -> [Sequence; 2] {
        [failed_render as Render, oversized_render].map(|render| {
            let mut sequence = Sequence::new("697+1209/1,silence/1", -12).unwrap();
            sequence.render = render;
            sequence
        })
    }

    #[test]
    fn generator_tone_provider_failures_are_reported() {
        unsafe {
            let mut api = rptadv_tone_descriptor().cast::<Descriptor>().read();
            api.create = Some(empty_create);
            assert!(Sequence::from_descriptor(&api.prefix, "1/1", 0).is_err());
        }
        for mut sequence in failing_sequences() {
            assert_eq!(sequence.render(&mut [0.0; 1]), Err(GeneratorError));
        }
    }
}
