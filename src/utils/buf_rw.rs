use super::ringbuf::SimpleFloatBuffer;

use std::cell::{Ref, RefMut};

// Unified interface to read audio data
// Internal - Used to read from buffers within an otters configuration
// External - Mostly for reading raw audio data from some source
pub enum AudioBufferReader<'a> {
    Null,
    Internal(Ref<'a, SimpleFloatBuffer>),
    External(*const f32),
}

// Same as AudioBufferReader, but for writing
pub enum AudioBufferWriter<'a> {
    Null,
    Internal(RefMut<'a, SimpleFloatBuffer>),
    External(*mut f32),
}

impl<'a> AudioBufferReader<'a> {
    pub fn buf_read(&'a self, idx: usize) -> f32 {
        match *self {
            AudioBufferReader::Null => 0.0f32,
            AudioBufferReader::Internal(ref flt_buf) => flt_buf.read(idx),
            AudioBufferReader::External(ptr) => unsafe_buf_read(ptr, idx),
        }
    }
}

impl<'a> AudioBufferWriter<'a> {
    pub fn buf_write(&mut self, idx: usize, value: f32) {
        match *self {
            AudioBufferWriter::Null => (),
            AudioBufferWriter::Internal(ref mut flt_buf) => flt_buf.write(value),
            AudioBufferWriter::External(ptr) => unsafe_buf_write(ptr, idx, value),
        }
    }
}

impl<'a> Default for AudioBufferReader<'a> {
    fn default() -> Self {
        return AudioBufferReader::Null;
    }
}

impl<'a> Default for AudioBufferWriter<'a> {
    fn default() -> Self {
        return AudioBufferWriter::Null;
    }
}

fn unsafe_buf_read(ptr: *const f32, idx: usize) -> f32 {
    if ptr.is_null() {
        return 0f32;
    }

    unsafe { *ptr.offset(idx as isize) }
}

fn unsafe_buf_write(ptr: *mut f32, idx: usize, value: f32) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        *ptr.offset(idx as isize) = value;
    }
}
