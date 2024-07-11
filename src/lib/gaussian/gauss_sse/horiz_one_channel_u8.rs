// Copyright (c) Radzivon Bartoshyk. All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
// 1.  Redistributions of source code must retain the above copyright notice, this
// list of conditions and the following disclaimer.
//
// 2.  Redistributions in binary form must reproduce the above copyright notice,
// this list of conditions and the following disclaimer in the documentation
// and/or other materials provided with the distribution.
//
// 3.  Neither the name of the copyright holder nor the names of its
// contributors may be used to endorse or promote products derived from
// this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::gaussian::gaussian_filter::GaussianFilter;
use crate::sse::{
    _mm_hsum_ps, _mm_loadu_ps_x2, _mm_loadu_ps_x4, _mm_loadu_si128_x2, _mm_prefer_fma_ps,
};
use crate::unsafe_slice::UnsafeSlice;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn gaussian_sse_horiz_one_chan_u8<T>(
    undef_src: &[T],
    src_stride: u32,
    undef_unsafe_dst: &UnsafeSlice<T>,
    dst_stride: u32,
    width: u32,
    kernel_size: usize,
    kernel: &[f32],
    start_y: u32,
    end_y: u32,
) {
    let src: &[u8] = unsafe { std::mem::transmute(undef_src) };
    let unsafe_dst: &UnsafeSlice<'_, u8> = unsafe { std::mem::transmute(undef_unsafe_dst) };
    let half_kernel = (kernel_size / 2) as i32;

    let mut _cy = start_y;

    let zeros_si = unsafe { _mm_setzero_si128() };

    for y in (_cy..end_y.saturating_sub(2)).step_by(2) {
        unsafe {
            for x in 0..width {
                let y_src_shift = y as usize * src_stride as usize;
                let y_dst_shift = y as usize * dst_stride as usize;

                let y_src_shift_next = y_src_shift + src_stride as usize;

                let mut store0 = _mm_setzero_ps();
                let mut store1 = _mm_setzero_ps();

                let mut r = -half_kernel;

                while r + 32 <= half_kernel && ((x as i64 + r as i64 + 32i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_u8x2 = _mm_loadu_si128_x2(s_ptr);
                    let pixel_colors_u8x2_next = _mm_loadu_si128_x2(s_ptr_next);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let weights0 = _mm_loadu_ps_x4(weight);
                    let weights1 = _mm_loadu_ps_x4(weight.add(16));

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color0, weights0.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color1, weights0.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color2, weights0.2);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color3, weights0.3);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color0, weights1.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color1, weights1.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color2, weights1.2);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color3, weights1.3);

                    // Next row

                    let pixel_colors_low_u16 =
                        _mm_unpacklo_epi8(pixel_colors_u8x2_next.0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color0, weights0.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color1, weights0.1);

                    let pixel_colors_high_u16 =
                        _mm_unpackhi_epi8(pixel_colors_u8x2_next.0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color2, weights0.2);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color3, weights0.3);

                    let pixel_colors_low_u16 =
                        _mm_unpacklo_epi8(pixel_colors_u8x2_next.1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color0, weights1.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color1, weights1.1);

                    let pixel_colors_high_u16 =
                        _mm_unpackhi_epi8(pixel_colors_u8x2_next.1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color2, weights1.2);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color3, weights1.3);

                    r += 32;
                }

                while r + 16 <= half_kernel && ((x as i64 + r as i64 + 16i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_u8_0 = _mm_loadu_si128(s_ptr as *const __m128i);
                    let pixel_colors_u8_1 = _mm_loadu_si128(s_ptr_next as *const __m128i);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let weights = _mm_loadu_ps_x4(weight);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8_0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color0, weights.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color1, weights.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8_0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color2, weights.2);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color3, weights.3);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8_1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color0, weights.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color1, weights.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8_1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color2, weights.2);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color3, weights.3);

                    r += 16;
                }

                while r + 8 <= half_kernel && ((x as i64 + r as i64 + 8i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_u16_0 = _mm_unpacklo_epi8(_mm_loadu_si64(s_ptr), zeros_si);
                    let pixel_colors_u16_1 =
                        _mm_unpacklo_epi8(_mm_loadu_si64(s_ptr_next), zeros_si);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let weights = _mm_loadu_ps_x2(weight);
                    let pixel_color_low =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_u16_0, zeros_si));
                    let pixel_color_high =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_u16_0, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color_low, weights.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color_high, weights.1);

                    let pixel_color_low =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_u16_1, zeros_si));
                    let pixel_color_high =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_u16_1, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color_low, weights.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color_high, weights.1);

                    r += 8;
                }

                while r + 4 <= half_kernel && ((x as i64 + r as i64 + 4i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_i32_0 = _mm_setr_epi32(
                        s_ptr.read_unaligned() as i32,
                        s_ptr.add(1).read_unaligned() as i32,
                        s_ptr.add(2).read_unaligned() as i32,
                        s_ptr.add(3).read_unaligned() as i32,
                    );

                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_i32_1 = _mm_setr_epi32(
                        s_ptr_next.read_unaligned() as i32,
                        s_ptr_next.add(1).read_unaligned() as i32,
                        s_ptr_next.add(2).read_unaligned() as i32,
                        s_ptr_next.add(3).read_unaligned() as i32,
                    );

                    let pixel_colors_f32_0 = _mm_cvtepi32_ps(pixel_colors_i32_0);
                    let pixel_colors_f32_1 = _mm_cvtepi32_ps(pixel_colors_i32_1);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let f_weight = _mm_loadu_ps(weight);
                    store0 = _mm_prefer_fma_ps(store0, pixel_colors_f32_0, f_weight);
                    store1 = _mm_prefer_fma_ps(store1, pixel_colors_f32_1, f_weight);

                    r += 4;
                }

                while r <= half_kernel {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_f32_0 =
                        _mm_setr_ps(s_ptr.read_unaligned() as f32, 0f32, 0f32, 0f32);
                    let pixel_colors_f32_1 =
                        _mm_setr_ps(s_ptr_next.read_unaligned() as f32, 0f32, 0f32, 0f32);
                    let weight = *kernel.get_unchecked((r + half_kernel) as usize);
                    let f_weight = _mm_set1_ps(weight);
                    store0 = _mm_prefer_fma_ps(store0, pixel_colors_f32_0, f_weight);
                    store1 = _mm_prefer_fma_ps(store1, pixel_colors_f32_1, f_weight);

                    r += 1;
                }

                let agg0 = _mm_hsum_ps(store0);
                let offset0 = y_dst_shift + x as usize;
                let dst_ptr0 = unsafe_dst.slice.as_ptr().add(offset0) as *mut u8;
                dst_ptr0.write_unaligned(agg0.round().min(255f32).max(0f32) as u8);

                let agg1 = _mm_hsum_ps(store1);
                let offset1 = offset0 + dst_stride as usize;
                let dst_ptr1 = unsafe_dst.slice.as_ptr().add(offset1) as *mut u8;
                dst_ptr1.write_unaligned(agg1.round().min(255f32).max(0f32) as u8);
            }
        }
        _cy = y;
    }

    for y in _cy..end_y {
        unsafe {
            for x in 0..width {
                let y_src_shift = y as usize * src_stride as usize;
                let y_dst_shift = y as usize * dst_stride as usize;

                let mut store = _mm_set1_ps(0f32);

                let mut r = -half_kernel;

                while r + 32 <= half_kernel && ((x as i64 + r as i64 + 32i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_u8x2 = _mm_loadu_si128_x2(s_ptr);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let weights0 = _mm_loadu_ps_x4(weight);
                    let weights1 = _mm_loadu_ps_x4(weight.add(16));

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color0, weights0.0);
                    store = _mm_prefer_fma_ps(store, pixel_color1, weights0.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color2, weights0.2);
                    store = _mm_prefer_fma_ps(store, pixel_color3, weights0.3);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color0, weights1.0);
                    store = _mm_prefer_fma_ps(store, pixel_color1, weights1.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color2, weights1.2);
                    store = _mm_prefer_fma_ps(store, pixel_color3, weights1.3);

                    r += 32;
                }

                while r + 16 <= half_kernel && ((x as i64 + r as i64 + 16i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_u8 = _mm_loadu_si128(s_ptr as *const __m128i);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let weights = _mm_loadu_ps_x4(weight);
                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color0, weights.0);
                    store = _mm_prefer_fma_ps(store, pixel_color1, weights.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color2, weights.2);
                    store = _mm_prefer_fma_ps(store, pixel_color3, weights.3);

                    r += 16;
                }

                while r + 8 <= half_kernel && ((x as i64 + r as i64 + 8i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_u16 = _mm_unpacklo_epi8(_mm_loadu_si64(s_ptr), zeros_si);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let weights = _mm_loadu_ps_x2(weight);
                    let pixel_color_low =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_u16, zeros_si));
                    let pixel_color_high =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color_low, weights.0);
                    store = _mm_prefer_fma_ps(store, pixel_color_high, weights.1);

                    r += 8;
                }

                while r + 4 <= half_kernel && ((x as i64 + r as i64 + 4i64) < width as i64) {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_i32 = _mm_setr_epi32(
                        s_ptr.read_unaligned() as i32,
                        s_ptr.add(1).read_unaligned() as i32,
                        s_ptr.add(2).read_unaligned() as i32,
                        s_ptr.add(3).read_unaligned() as i32,
                    );
                    let pixel_colors_f32 = _mm_cvtepi32_ps(pixel_colors_i32);
                    let weight = kernel.as_ptr().add((r + half_kernel) as usize);
                    let f_weight = _mm_loadu_ps(weight);
                    store = _mm_prefer_fma_ps(store, pixel_colors_f32, f_weight);

                    r += 4;
                }

                while r <= half_kernel {
                    let current_x =
                        std::cmp::min(std::cmp::max(x as i64 + r as i64, 0), (width - 1) as i64)
                            as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let value = s_ptr.read_unaligned() as f32;
                    let pixel_colors_f32 = _mm_setr_ps(value, 0f32, 0f32, 0f32);
                    let weight = *kernel.get_unchecked((r + half_kernel) as usize);
                    let f_weight = _mm_setr_ps(weight, 0f32, 0f32, 0f32);
                    store = _mm_prefer_fma_ps(store, pixel_colors_f32, f_weight);

                    r += 1;
                }

                let agg = _mm_hsum_ps(store);
                let offset = y_dst_shift + x as usize;
                let dst_ptr = unsafe_dst.slice.as_ptr().add(offset) as *mut u8;
                dst_ptr.write_unaligned(agg.round().min(255f32).max(0f32) as u8);
            }
        }
    }
}

pub fn gaussian_sse_horiz_one_chan_filter_u8<T>(
    undef_src: &[T],
    src_stride: u32,
    undef_unsafe_dst: &UnsafeSlice<T>,
    dst_stride: u32,
    width: u32,
    filter: &Vec<GaussianFilter>,
    start_y: u32,
    end_y: u32,
) {
    let src: &[u8] = unsafe { std::mem::transmute(undef_src) };
    let unsafe_dst: &UnsafeSlice<'_, u8> = unsafe { std::mem::transmute(undef_unsafe_dst) };

    let mut _cy = start_y;

    let zeros_si = unsafe { _mm_setzero_si128() };

    for y in (_cy..end_y.saturating_sub(2)).step_by(2) {
        unsafe {
            for x in 0..width {
                let current_filter = filter.get_unchecked(x as usize);
                let filter_start = current_filter.start;
                let filter_weights = &current_filter.filter;

                let y_src_shift = y as usize * src_stride as usize;
                let y_dst_shift = y as usize * dst_stride as usize;

                let y_src_shift_next = y_src_shift + src_stride as usize;

                let mut store0 = _mm_setzero_ps();
                let mut store1 = _mm_setzero_ps();

                let mut r = 0;

                while r + 32 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 32i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_u8x2 = _mm_loadu_si128_x2(s_ptr);
                    let pixel_colors_u8x2_next = _mm_loadu_si128_x2(s_ptr_next);
                    let weight = filter_weights.as_ptr().add(r);
                    let weights0 = _mm_loadu_ps_x4(weight);
                    let weights1 = _mm_loadu_ps_x4(weight.add(16));

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color0, weights0.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color1, weights0.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color2, weights0.2);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color3, weights0.3);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color0, weights1.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color1, weights1.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color2, weights1.2);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color3, weights1.3);

                    // Next row

                    let pixel_colors_low_u16 =
                        _mm_unpacklo_epi8(pixel_colors_u8x2_next.0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color0, weights0.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color1, weights0.1);

                    let pixel_colors_high_u16 =
                        _mm_unpackhi_epi8(pixel_colors_u8x2_next.0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color2, weights0.2);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color3, weights0.3);

                    let pixel_colors_low_u16 =
                        _mm_unpacklo_epi8(pixel_colors_u8x2_next.1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color0, weights1.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color1, weights1.1);

                    let pixel_colors_high_u16 =
                        _mm_unpackhi_epi8(pixel_colors_u8x2_next.1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color2, weights1.2);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color3, weights1.3);

                    r += 32;
                }

                while r + 16 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 16i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_u8_0 = _mm_loadu_si128(s_ptr as *const __m128i);
                    let pixel_colors_u8_1 = _mm_loadu_si128(s_ptr_next as *const __m128i);
                    let weight = filter_weights.as_ptr().add(r);
                    let weights = _mm_loadu_ps_x4(weight);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8_0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color0, weights.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color1, weights.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8_0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color2, weights.2);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color3, weights.3);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8_1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color0, weights.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color1, weights.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8_1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color2, weights.2);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color3, weights.3);

                    r += 16;
                }

                while r + 8 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 8i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_u16_0 = _mm_unpacklo_epi8(_mm_loadu_si64(s_ptr), zeros_si);
                    let pixel_colors_u16_1 =
                        _mm_unpacklo_epi8(_mm_loadu_si64(s_ptr_next), zeros_si);
                    let weight = filter_weights.as_ptr().add(r);
                    let weights = _mm_loadu_ps_x2(weight);
                    let pixel_color_low =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_u16_0, zeros_si));
                    let pixel_color_high =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_u16_0, zeros_si));
                    store0 = _mm_prefer_fma_ps(store0, pixel_color_low, weights.0);
                    store0 = _mm_prefer_fma_ps(store0, pixel_color_high, weights.1);

                    let pixel_color_low =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_u16_1, zeros_si));
                    let pixel_color_high =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_u16_1, zeros_si));
                    store1 = _mm_prefer_fma_ps(store1, pixel_color_low, weights.0);
                    store1 = _mm_prefer_fma_ps(store1, pixel_color_high, weights.1);

                    r += 8;
                }

                while r + 4 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 4i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_i32_0 = _mm_setr_epi32(
                        s_ptr.read_unaligned() as i32,
                        s_ptr.add(1).read_unaligned() as i32,
                        s_ptr.add(2).read_unaligned() as i32,
                        s_ptr.add(3).read_unaligned() as i32,
                    );

                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_i32_1 = _mm_setr_epi32(
                        s_ptr_next.read_unaligned() as i32,
                        s_ptr_next.add(1).read_unaligned() as i32,
                        s_ptr_next.add(2).read_unaligned() as i32,
                        s_ptr_next.add(3).read_unaligned() as i32,
                    );

                    let pixel_colors_f32_0 = _mm_cvtepi32_ps(pixel_colors_i32_0);
                    let pixel_colors_f32_1 = _mm_cvtepi32_ps(pixel_colors_i32_1);
                    let weight = filter_weights.as_ptr().add(r);
                    let f_weight = _mm_loadu_ps(weight);
                    store0 = _mm_prefer_fma_ps(store0, pixel_colors_f32_0, f_weight);
                    store1 = _mm_prefer_fma_ps(store1, pixel_colors_f32_1, f_weight);

                    r += 4;
                }

                while r < current_filter.size {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let s_ptr_next = src.as_ptr().add(y_src_shift_next + px);
                    let pixel_colors_f32_0 =
                        _mm_setr_ps(s_ptr.read_unaligned() as f32, 0f32, 0f32, 0f32);
                    let pixel_colors_f32_1 =
                        _mm_setr_ps(s_ptr_next.read_unaligned() as f32, 0f32, 0f32, 0f32);
                    let weight = filter_weights.as_ptr().add(r).read_unaligned();
                    let f_weight = _mm_setr_ps(weight, 0f32, 0f32, 0f32);
                    store0 = _mm_prefer_fma_ps(store0, pixel_colors_f32_0, f_weight);
                    store1 = _mm_prefer_fma_ps(store1, pixel_colors_f32_1, f_weight);

                    r += 1;
                }

                let agg0 = _mm_hsum_ps(store0);
                let offset0 = y_dst_shift + x as usize;
                let dst_ptr0 = unsafe_dst.slice.as_ptr().add(offset0) as *mut u8;
                dst_ptr0.write_unaligned(agg0.min(255f32).max(0f32) as u8);

                let agg1 = _mm_hsum_ps(store1);
                let offset1 = offset0 + dst_stride as usize;
                let dst_ptr1 = unsafe_dst.slice.as_ptr().add(offset1) as *mut u8;
                dst_ptr1.write_unaligned(agg1.min(255f32).max(0f32) as u8);
            }
        }
        _cy = y;
    }

    for y in _cy..end_y {
        unsafe {
            for x in 0..width {
                let y_src_shift = y as usize * src_stride as usize;
                let y_dst_shift = y as usize * dst_stride as usize;

                let current_filter = filter.get_unchecked(x as usize);
                let filter_start = current_filter.start;
                let filter_weights = &current_filter.filter;

                let mut store = _mm_setzero_ps();

                let mut r = 0;

                while r + 32 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 32i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_u8x2 = _mm_loadu_si128_x2(s_ptr);
                    let weight = filter_weights.as_ptr().add(r);
                    let weights0 = _mm_loadu_ps_x4(weight);
                    let weights1 = _mm_loadu_ps_x4(weight.add(16));

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color0, weights0.0);
                    store = _mm_prefer_fma_ps(store, pixel_color1, weights0.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.0, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color2, weights0.2);
                    store = _mm_prefer_fma_ps(store, pixel_color3, weights0.3);

                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color0, weights1.0);
                    store = _mm_prefer_fma_ps(store, pixel_color1, weights1.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8x2.1, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color2, weights1.2);
                    store = _mm_prefer_fma_ps(store, pixel_color3, weights1.3);

                    r += 32;
                }

                while r + 16 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 16i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_u8 = _mm_loadu_si128(s_ptr as *const __m128i);
                    let weight = filter_weights.as_ptr().add(r);
                    let weights = _mm_loadu_ps_x4(weight);
                    let pixel_colors_low_u16 = _mm_unpacklo_epi8(pixel_colors_u8, zeros_si);
                    let pixel_color0 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_low_u16, zeros_si));
                    let pixel_color1 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_low_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color0, weights.0);
                    store = _mm_prefer_fma_ps(store, pixel_color1, weights.1);

                    let pixel_colors_high_u16 = _mm_unpackhi_epi8(pixel_colors_u8, zeros_si);
                    let pixel_color2 =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_high_u16, zeros_si));
                    let pixel_color3 =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_high_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color2, weights.2);
                    store = _mm_prefer_fma_ps(store, pixel_color3, weights.3);

                    r += 16;
                }

                while r + 8 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 8i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_u16 =
                        _mm_unpacklo_epi8(_mm_loadu_si64(s_ptr), _mm_setzero_si128());
                    let weight = filter_weights.as_ptr().add(r);
                    let weights = _mm_loadu_ps_x2(weight);
                    let pixel_color_low =
                        _mm_cvtepi32_ps(_mm_unpacklo_epi16(pixel_colors_u16, zeros_si));
                    let pixel_color_high =
                        _mm_cvtepi32_ps(_mm_unpackhi_epi16(pixel_colors_u16, zeros_si));
                    store = _mm_prefer_fma_ps(store, pixel_color_low, weights.0);
                    store = _mm_prefer_fma_ps(store, pixel_color_high, weights.1);

                    r += 8;
                }

                while r + 4 < current_filter.size
                    && ((filter_start as i64 + r as i64 + 4i64) < width as i64)
                {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let pixel_colors_i32 = _mm_setr_epi32(
                        s_ptr.read_unaligned() as i32,
                        s_ptr.add(1).read_unaligned() as i32,
                        s_ptr.add(2).read_unaligned() as i32,
                        s_ptr.add(3).read_unaligned() as i32,
                    );
                    let pixel_colors_f32 = _mm_cvtepi32_ps(pixel_colors_i32);
                    let weight = filter_weights.as_ptr().add(r);
                    let f_weight = _mm_loadu_ps(weight);
                    store = _mm_prefer_fma_ps(store, pixel_colors_f32, f_weight);

                    r += 4;
                }

                while r < current_filter.size {
                    let current_x = std::cmp::min(
                        std::cmp::max(filter_start as i64 + r as i64, 0),
                        (width - 1) as i64,
                    ) as usize;
                    let px = current_x;
                    let s_ptr = src.as_ptr().add(y_src_shift + px);
                    let value = s_ptr.read_unaligned() as f32;
                    let pixel_colors_f32 = _mm_setr_ps(value, 0f32, 0f32, 0f32);
                    let weight = filter_weights.as_ptr().add(r).read_unaligned();
                    let f_weight = _mm_set1_ps(weight);
                    store = _mm_prefer_fma_ps(store, pixel_colors_f32, f_weight);

                    r += 1;
                }

                let agg = _mm_hsum_ps(store);
                let offset = y_dst_shift + x as usize;
                let dst_ptr = unsafe_dst.slice.as_ptr().add(offset) as *mut u8;
                dst_ptr.write_unaligned(agg.round().min(255f32).max(0f32) as u8);
            }
        }
    }
}
