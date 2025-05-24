// Copyright (c) 2011 Jan Kokemüller
// Copyright (c) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use crate::utils::FrameAccumulator;
use std::f64::consts::PI;

const ALMOST_ZERO: f64 = 0.000001;
const TAPS: usize = 48;

/// A circular buffer offering fixed-length continous views into data
/// This is enabled by writing data twice, also to a "shadow"-buffer following the primary buffer,
/// The tradeoff is writing all data twice, the gain is giving the compiler continuous view with
/// predictable length into the data, unlocking some more optimizations
#[derive(Clone, Debug)]
struct RollingBuffer<T, const N: usize> {
    buf: [T; TAPS],
    position: usize,
}

impl<T: Default + Copy, const N: usize> RollingBuffer<T, N> {
    fn new() -> Self {
        assert!(N * 2 <= TAPS);

        let buf: [T; TAPS] = [Default::default(); TAPS];

        Self { buf, position: N }
    }

    #[inline(always)]
    fn push_front(&mut self, v: T) {
        if self.position == 0 {
            self.position = N - 1;
        } else {
            self.position -= 1;
        }
        // this is safe, since self.position is always kept below N, which is checked at creation
        // to be `<= buf.size() / 2`
        unsafe {
            *self.buf.get_unchecked_mut(self.position) = v;
            *self.buf.get_unchecked_mut(self.position + N) = v;
        }
    }
}

impl<T, const N: usize> AsRef<[T; N]> for RollingBuffer<T, N> {
    #[inline(always)]
    fn as_ref(&self) -> &[T; N] {
        // this is safe, since self.position is always kept below N, which is checked at creation
        // to be `<= buf.size() / 2`
        unsafe { &*(self.buf.get_unchecked(self.position) as *const T as *const [T; N]) }
    }
}

#[derive(Debug, Clone)]
pub struct InterpF<const ACTIVE_TAPS: usize, const FACTOR: usize, F: FrameAccumulator> {
    filter: [[f32; FACTOR]; ACTIVE_TAPS],
    buffer: RollingBuffer<F, ACTIVE_TAPS>,
}

impl<const ACTIVE_TAPS: usize, const FACTOR: usize, F> Default for InterpF<ACTIVE_TAPS, FACTOR, F>
where
    F: FrameAccumulator + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const ACTIVE_TAPS: usize, const FACTOR: usize, F> InterpF<ACTIVE_TAPS, FACTOR, F>
where
    F: FrameAccumulator + Default,
{
    pub fn new() -> Self {
        assert_eq!(ACTIVE_TAPS * FACTOR, TAPS);

        let mut filter: [[_; FACTOR]; ACTIVE_TAPS] = [[0f32; FACTOR]; ACTIVE_TAPS];
        for (j, coeff) in filter.iter_mut().flat_map(|x| x.iter_mut()).enumerate() {
            let j = j as f64;
            // Calculate Hanning window,
            let window = TAPS + 1;
            // Ignore one tap. (Last tap is zero anyways, and we want to hit an even multiple of 48)
            let window = (window - 1) as f64;
            let w = 0.5 * (1.0 - f64::cos(2.0 * PI * j / window));

            // Calculate sinc and apply hanning window
            let m = j - window / 2.0;
            *coeff = if m.abs() > ALMOST_ZERO {
                w * f64::sin(m * PI / FACTOR as f64) / (m * PI / FACTOR as f64)
            } else {
                w
            } as f32;
        }

        Self {
            filter,
            buffer: RollingBuffer::new(),
        }
    }

    pub fn interpolate(&mut self, frame: F) -> [F; FACTOR] {
        // Write in Frames in reverse, to enable forward-scanning with filter
        self.buffer.push_front(frame);

        let mut output: [F; FACTOR] = [Default::default(); FACTOR];

        let buf = self.buffer.as_ref();

        for (filter_coeffs, input_frame) in Iterator::zip(self.filter.iter(), buf) {
            for (output_frame, coeff) in Iterator::zip(output.iter_mut(), filter_coeffs) {
                output_frame.scale_add(input_frame, *coeff);
            }
        }

        output
    }

    pub fn reset(&mut self) {
        self.buffer = RollingBuffer::new();
    }
}