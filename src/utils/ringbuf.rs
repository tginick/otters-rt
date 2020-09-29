use super::mathutils::is_power_of_2;

use std::cmp;

use fftw::array::AlignedVec;


// This is a special kind of ring buf that has both a read and write ptr
// its size must be a power of 2
pub struct FFTCollectionBuffer {
    data: AlignedVec<f32>,

    // used to wrap index without a conditional
    // technique only works if size is a power of 2
    index_wrap_mask: usize,
    read_idx: usize,
    write_idx: usize,
}

pub struct SimpleFloatBuffer {
    data: Vec<f32>,
    capacity: usize,
    limit: usize,
    write_idx: usize,
}

pub struct TinyFloatBuffer {
    x: [f32; 2],
    next_idx: usize,
}

impl FFTCollectionBuffer {
    pub fn new(length: usize) -> Option<FFTCollectionBuffer> {
        if !is_power_of_2(length) {
            return None;
        }
        
        let mut buf = FFTCollectionBuffer {
            data: AlignedVec::new(length),
            index_wrap_mask: length - 1,
            read_idx: 0,
            write_idx: 0,
        };

        for i in 0..length {
            buf.data[i] = 0_f32;
        }

        Some(buf)
    }

    pub fn get_read_idx(&self) -> usize {
        self.read_idx
    }

    pub fn get_write_idx(&self) -> usize {
        self.write_idx
    }

    pub fn set_read_idx(&mut self, new_idx: usize) {
        self.read_idx = new_idx;
        if self.read_idx >= self.data.len() {
            self.read_idx = self.data.len() - 1;
        }
    }

    pub fn set_write_idx(&mut self, new_idx: usize) {
        self.write_idx = new_idx;
        if self.write_idx >= self.data.len() {
            self.write_idx = self.data.len() - 1;
        }
    }

    pub fn advance_read_idx(&mut self) {
        self.read_idx += 1;
        self.read_idx &= self.index_wrap_mask;
    }

    pub fn rewind_read_idx(&mut self, count: usize) {
        if count <= self.read_idx {
            self.read_idx -= count; 
        } else {
            let new_count = count - self.read_idx - 1;
            self.read_idx = self.data.len() - 1 - new_count;
        }
    }

    pub fn rewind_write_idx(&mut self, count: usize) {
        if count <= self.write_idx {
            self.write_idx -= count; 
        } else {
            let new_count = count - self.write_idx - 1;
            self.write_idx = self.data.len() - 1 - new_count;
        }
    }

    pub fn advance_write_idx(&mut self) {
        self.write_idx += 1;
        self.write_idx &= self.index_wrap_mask;
    }

    pub fn advance_both_idx(&mut self) {
        self.advance_read_idx();
        self.advance_write_idx();
    }

    pub fn get_at_idx(&self, idx: usize) -> f32 {
        self.data[idx]
    }

    pub fn set_at_idx(&mut self, idx: usize, value: f32) {
        self.data[idx] = value;
    }

    pub fn get_at_read_idx(&self) -> f32 {
        self.get_at_idx(self.get_read_idx())
    }

    pub fn set_at_write_idx(&mut self, value: f32) {
        self.set_at_idx(self.get_write_idx(), value);
    }
}

impl SimpleFloatBuffer {
    pub fn with_max_capacity(capacity: usize) -> SimpleFloatBuffer {
        let mut zeroed_data = Vec::with_capacity(capacity);

        // every idx in this buffer is always valid (may be 0)
        for _ in 0..capacity {
            zeroed_data.push(0.0f32);
        }

        SimpleFloatBuffer {
            data: zeroed_data,
            capacity,
            limit: capacity,
            write_idx: 0,
        }
    }

    pub fn get_capacity(&self) -> usize {
        self.capacity
    }

    pub fn get_limit(&self) -> usize {
        self.limit
    }

    pub fn set_limit(&mut self, new_limit: usize) {
        self.limit = cmp::min(self.capacity, new_limit);
    }

    pub fn write(&mut self, value: f32) {
        self.data[self.write_idx] = value;
        self.write_idx = (self.write_idx + 1) % self.limit;
    }

    pub fn clear(&mut self) {
        for i in 0..self.capacity {
            self.data[i] = 0.0f32;
        }

        self.write_idx = 0;
    }

    pub fn read(&self, idx: usize) -> f32 {
        // TODO
        self.data[(self.write_idx + idx) % self.limit]
    }
}

impl TinyFloatBuffer {
    pub fn new() -> TinyFloatBuffer {
        let x: [f32; 2] = [0f32, 0f32];

        TinyFloatBuffer { x, next_idx: 0 }
    }

    pub fn z2(&self) -> f32 {
        return self.x[self.next_idx];
    }

    pub fn z1(&self) -> f32 {
        let prev_idx = (self.next_idx + 1) % 2;
        return self.x[prev_idx];
    }

    pub fn write(&mut self, v: f32) {
        self.x[self.next_idx] = v;
        self.next_idx = (self.next_idx + 1) % 2;
    }
}
