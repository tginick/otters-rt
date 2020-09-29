#![allow(dead_code)]

use std::cell::Cell;

// based on https://github.com/lemire/testingRNG/blob/master/source/wyhash.h
// used under apache license
pub struct WyHashPRNG {
    state: Cell<u64>,
}

impl WyHashPRNG {
    pub fn new(initial_state: u64) -> WyHashPRNG {
        WyHashPRNG {
            state: Cell::new(initial_state),
        }
    }

    pub fn get_state(&self) -> u64 {
        self.state.get()
    }

    pub fn set_state(&mut self, state: u64) {
        self.state.set(state);
    }

    pub fn next(&self) -> u64 {
        let mut state = self.state.get();

        state += 0x60bee2bee120fc15;

        self.state.set(state);
        let mut tmp: u128 = state as u128;

        tmp *= 0xa3b195354a39b70d;

        let m1 = ((tmp >> 64) ^ tmp) as u64;

        tmp = (m1 as u128) * 0x1b03738712fad5c9;

        let m2 = ((tmp >> 64) ^ tmp) as u64;

        m2
    }
}