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

use crate::fast_gaussian_next_f16::fast_gaussian_next_f16;
use crate::fast_gaussian_next_f32::fast_gaussian_next_f32;
#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
use crate::neon::{
    fast_gaussian_next_horizontal_pass_neon_u8, fast_gaussian_next_vertical_pass_neon_u8,
};
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "x86"),
    target_feature = "sse4.1"
))]
use crate::sse::{
    fast_gaussian_next_horizontal_pass_sse_u8, fast_gaussian_next_vertical_pass_sse_u8,
};
use crate::unsafe_slice::UnsafeSlice;
use crate::{FastBlurChannels, ThreadingPolicy};
use colorutils_rs::{
    linear_to_rgb, linear_to_rgba, rgb_to_linear, rgba_to_linear, TransferFunction,
};
use num_traits::{AsPrimitive, FromPrimitive};

const BASE_RADIUS_I64_CUTOFF: u32 = 125;

fn fast_gaussian_next_vertical_pass<
    T: FromPrimitive + Default + Into<i32>,
    J,
    const CHANNEL_CONFIGURATION: usize,
>(
    bytes: &UnsafeSlice<T>,
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    start: u32,
    end: u32,
) where
    T: std::ops::AddAssign
        + std::ops::SubAssign
        + Copy
        + FromPrimitive
        + Default
        + Into<i32>
        + Into<J>,
    J: Copy
        + FromPrimitive
        + AsPrimitive<f64>
        + Default
        + Into<i64>
        + std::ops::Mul<Output = J>
        + std::ops::Sub<Output = J>
        + std::ops::Add<Output = J>
        + std::ops::AddAssign
        + std::ops::SubAssign
        + From<i32>,
    i64: From<T>,
{
    let mut buffer_r: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let mut buffer_g: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let mut buffer_b: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let mut buffer_a: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let radius_64 = radius as i64;
    let height_wide = height as i64;
    let weight = 1.0f64 / ((radius as f64) * (radius as f64) * (radius as f64));
    for x in start..std::cmp::min(width, end) {
        let mut dif_r: J = J::from_i32(0i32).unwrap();
        let mut der_r: J = J::from_i32(0i32).unwrap();
        let mut sum_r: J = J::from_i32(0i32).unwrap();
        let mut dif_g: J = J::from_i32(0i32).unwrap();
        let mut der_g: J = J::from_i32(0i32).unwrap();
        let mut sum_g: J = J::from_i32(0i32).unwrap();
        let mut dif_b: J = J::from_i32(0i32).unwrap();
        let mut der_b: J = J::from_i32(0i32).unwrap();
        let mut sum_b: J = J::from_i32(0i32).unwrap();
        let mut dif_a: J = J::from_i32(0i32).unwrap();
        let mut der_a: J = J::from_i32(0i32).unwrap();
        let mut sum_a: J = J::from_i32(0i32).unwrap();

        let current_px = (x * CHANNEL_CONFIGURATION as u32) as usize;

        let start_y = 0 - 3 * radius as i64;
        for y in start_y..height_wide {
            let current_y = (y * (stride as i64)) as usize;
            if y >= 0 {
                let sum_r_f: f64 = sum_r.as_();
                let sum_g_f: f64 = sum_g.as_();
                let sum_b_f: f64 = sum_b.as_();
                let new_r = T::from_u32((sum_r_f * weight) as u32).unwrap_or_default();
                let new_g = T::from_u32((sum_g_f * weight) as u32).unwrap_or_default();
                let new_b = T::from_u32((sum_b_f * weight) as u32).unwrap_or_default();

                let bytes_offset = current_y + current_px;

                unsafe {
                    bytes.write(bytes_offset, new_r);
                    bytes.write(bytes_offset + 1, new_g);
                    bytes.write(bytes_offset + 2, new_b);
                    if CHANNEL_CONFIGURATION == 4 {
                        let sum_a_f: f64 = sum_a.as_();
                        let new_a = T::from_u32((sum_a_f * weight) as u32).unwrap_or_default();
                        bytes.write(bytes_offset + 3, new_a);
                    }
                }

                let d_arr_index_1 = ((y + radius_64) & 1023) as usize;
                let d_arr_index_2 = ((y - radius_64) & 1023) as usize;
                let d_arr_index = (y & 1023) as usize;
                let threes = J::from_i32(3i32).unwrap();
                dif_r += threes
                    * (unsafe { *buffer_r.get_unchecked(d_arr_index) }
                        - unsafe { *buffer_r.get_unchecked(d_arr_index_1) })
                    - unsafe { *buffer_r.get_unchecked(d_arr_index_2) };
                dif_g += threes
                    * (unsafe { *buffer_g.get_unchecked(d_arr_index) }
                        - unsafe { *buffer_g.get_unchecked(d_arr_index_1) })
                    - unsafe { *buffer_g.get_unchecked(d_arr_index_2) };
                dif_b += threes
                    * (unsafe { *buffer_b.get_unchecked(d_arr_index) }
                        - unsafe { *buffer_b.get_unchecked(d_arr_index_1) })
                    - unsafe { *buffer_b.get_unchecked(d_arr_index_2) };
                if CHANNEL_CONFIGURATION == 4 {
                    dif_a += threes
                        * (unsafe { *buffer_a.get_unchecked(d_arr_index) }
                            - unsafe { *buffer_a.get_unchecked(d_arr_index_1) })
                        - unsafe { *buffer_a.get_unchecked(d_arr_index_2) };
                }
            } else if y + radius_64 >= 0 {
                let arr_index = (y & 1023) as usize;
                let arr_index_1 = ((y + radius_64) & 1023) as usize;
                let threes = J::from_i32(3i32).unwrap();
                dif_r += threes
                    * (unsafe { *buffer_r.get_unchecked(arr_index) }
                        - unsafe { *buffer_r.get_unchecked(arr_index_1) });
                dif_g += threes
                    * (unsafe { *buffer_g.get_unchecked(arr_index) }
                        - unsafe { *buffer_g.get_unchecked(arr_index_1) });
                dif_b += threes
                    * (unsafe { *buffer_b.get_unchecked(arr_index) }
                        - unsafe { *buffer_b.get_unchecked(arr_index_1) });
                if CHANNEL_CONFIGURATION == 4 {
                    dif_a += threes
                        * (unsafe { *buffer_a.get_unchecked(arr_index) }
                            - unsafe { *buffer_a.get_unchecked(arr_index_1) });
                }
            } else if y + 2 * radius_64 >= 0 {
                let arr_index = ((y + radius_64) & 1023) as usize;
                let threes = J::from_i32(3i32).unwrap();
                dif_r -= threes * unsafe { *buffer_r.get_unchecked(arr_index) };
                dif_g -= threes * unsafe { *buffer_g.get_unchecked(arr_index) };
                dif_b -= threes * unsafe { *buffer_b.get_unchecked(arr_index) };
                if CHANNEL_CONFIGURATION == 4 {
                    dif_a -= threes * unsafe { *buffer_a.get_unchecked(arr_index) };
                }
            }

            let next_row_y = (std::cmp::min(
                std::cmp::max(y + ((3 * radius_64) >> 1), 0),
                height_wide - 1,
            ) as usize)
                * (stride as usize);
            let next_row_x = (x * CHANNEL_CONFIGURATION as u32) as usize;

            let px_idx = next_row_y + next_row_x;

            let ur8 = bytes[px_idx];
            let ug8 = bytes[px_idx + 1];
            let ub8 = bytes[px_idx + 2];

            let arr_index = ((y + 2 * radius_64) & 1023) as usize;

            dif_r += ur8.into();
            der_r += dif_r;
            sum_r += der_r;
            unsafe {
                *buffer_r.get_unchecked_mut(arr_index) = ur8.into();
            }

            dif_g += ug8.into();
            der_g += dif_g;
            sum_g += der_g;
            unsafe {
                *buffer_g.get_unchecked_mut(arr_index) = ug8.into();
            }

            dif_b += ub8.into();
            der_b += dif_b;
            sum_b += der_b;
            unsafe {
                *buffer_b.get_unchecked_mut(arr_index) = ub8.into();
            }

            if CHANNEL_CONFIGURATION == 4 {
                let ua8 = bytes[px_idx + 3];

                dif_a += ua8.into();
                der_a += dif_a;
                sum_a += der_a;
                unsafe {
                    *buffer_a.get_unchecked_mut(arr_index) = ua8.into();
                }
            }
        }
    }
}

fn fast_gaussian_next_horizontal_pass<
    T: FromPrimitive + Default + Into<i32> + Send + Sync,
    J,
    const CHANNEL_CONFIGURATION: usize,
>(
    bytes: &UnsafeSlice<T>,
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    start: u32,
    end: u32,
) where
    T: std::ops::AddAssign
        + std::ops::SubAssign
        + Copy
        + FromPrimitive
        + Default
        + Into<i32>
        + Into<J>,
    J: Copy
        + FromPrimitive
        + AsPrimitive<f64>
        + Default
        + Into<i64>
        + std::ops::Mul<Output = J>
        + std::ops::Sub<Output = J>
        + std::ops::Add<Output = J>
        + std::ops::AddAssign
        + std::ops::SubAssign
        + From<i32>,
    i64: From<T>,
{
    let mut buffer_r: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let mut buffer_g: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let mut buffer_b: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let mut buffer_a: [J; 1024] = [J::from_i32(0i32).unwrap(); 1024];
    let radius_64 = radius as i64;
    let width_wide = width as i64;
    let weight = 1.0f64 / ((radius as f64) * (radius as f64) * (radius as f64));
    for y in start..std::cmp::min(height, end) {
        let mut dif_r: J = J::from_i32(0i32).unwrap();
        let mut der_r: J = J::from_i32(0i32).unwrap();
        let mut sum_r: J = J::from_i32(0i32).unwrap();
        let mut dif_g: J = J::from_i32(0i32).unwrap();
        let mut der_g: J = J::from_i32(0i32).unwrap();
        let mut sum_g: J = J::from_i32(0i32).unwrap();
        let mut dif_b: J = J::from_i32(0i32).unwrap();
        let mut der_b: J = J::from_i32(0i32).unwrap();
        let mut sum_b: J = J::from_i32(0i32).unwrap();
        let mut dif_a: J = J::from_i32(0i32).unwrap();
        let mut der_a: J = J::from_i32(0i32).unwrap();
        let mut sum_a: J = J::from_i32(0i32).unwrap();

        let current_y = ((y as i64) * (stride as i64)) as usize;

        for x in (0 - 3 * radius_64)..(width as i64) {
            if x >= 0 {
                let current_px =
                    ((std::cmp::max(x, 0) as u32) * CHANNEL_CONFIGURATION as u32) as usize;
                let sum_r_f: f64 = sum_r.as_();
                let sum_g_f: f64 = sum_g.as_();
                let sum_b_f: f64 = sum_b.as_();
                let new_r = T::from_u32((sum_r_f * weight).round() as u32).unwrap_or_default();
                let new_g = T::from_u32((sum_g_f * weight).round() as u32).unwrap_or_default();
                let new_b = T::from_u32((sum_b_f * weight).round() as u32).unwrap_or_default();

                let bytes_offset = current_y + current_px;

                unsafe {
                    bytes.write(bytes_offset, new_r);
                    bytes.write(bytes_offset + 1, new_g);
                    bytes.write(bytes_offset + 2, new_b);
                    if CHANNEL_CONFIGURATION == 4 {
                        let sum_a_f: f64 = sum_a.as_();
                        let new_a =
                            T::from_u32((sum_a_f * weight).round() as u32).unwrap_or_default();
                        bytes.write(bytes_offset + 3, new_a);
                    }
                }

                let d_arr_index_1 = ((x + radius_64) & 1023) as usize;
                let d_arr_index_2 = ((x - radius_64) & 1023) as usize;
                let d_arr_index = (x & 1023) as usize;
                let threes = J::from_i32(3i32).unwrap();
                dif_r += threes
                    * (unsafe { *buffer_r.get_unchecked(d_arr_index) }
                        - unsafe { *buffer_r.get_unchecked(d_arr_index_1) })
                    - unsafe { *buffer_r.get_unchecked(d_arr_index_2) };
                dif_g += threes
                    * (unsafe { *buffer_g.get_unchecked(d_arr_index) }
                        - unsafe { *buffer_g.get_unchecked(d_arr_index_1) })
                    - unsafe { *buffer_g.get_unchecked(d_arr_index_2) };
                dif_b += threes
                    * (unsafe { *buffer_b.get_unchecked(d_arr_index) }
                        - unsafe { *buffer_b.get_unchecked(d_arr_index_1) })
                    - unsafe { *buffer_b.get_unchecked(d_arr_index_2) };
                if CHANNEL_CONFIGURATION == 4 {
                    dif_a += threes
                        * (unsafe { *buffer_a.get_unchecked(d_arr_index) }
                            - unsafe { *buffer_a.get_unchecked(d_arr_index_1) })
                        - unsafe { *buffer_a.get_unchecked(d_arr_index_2) };
                }
            } else if x + radius_64 >= 0 {
                let arr_index = (x & 1023) as usize;
                let arr_index_1 = ((x + radius_64) & 1023) as usize;
                let threes = J::from_i32(3i32).unwrap();
                dif_r += threes
                    * (unsafe { *buffer_r.get_unchecked(arr_index) }
                        - unsafe { *buffer_r.get_unchecked(arr_index_1) });
                dif_g += threes
                    * (unsafe { *buffer_g.get_unchecked(arr_index) }
                        - unsafe { *buffer_g.get_unchecked(arr_index_1) });
                dif_b += threes
                    * (unsafe { *buffer_b.get_unchecked(arr_index) }
                        - unsafe { *buffer_b.get_unchecked(arr_index_1) });
                if CHANNEL_CONFIGURATION == 4 {
                    dif_a += threes
                        * (unsafe { *buffer_a.get_unchecked(arr_index) }
                            - unsafe { *buffer_a.get_unchecked(arr_index_1) });
                }
            } else if x + 2 * radius_64 >= 0 {
                let arr_index = ((x + radius_64) & 1023) as usize;
                let threes = J::from_i32(3i32).unwrap();
                dif_r -= threes * unsafe { *buffer_r.get_unchecked(arr_index) };
                dif_g -= threes * unsafe { *buffer_g.get_unchecked(arr_index) };
                dif_b -= threes * unsafe { *buffer_b.get_unchecked(arr_index) };
                if CHANNEL_CONFIGURATION == 4 {
                    dif_a -= threes * unsafe { *buffer_a.get_unchecked(arr_index) };
                }
            }

            let next_row_y = (y as usize) * (stride as usize);
            let next_row_x =
                ((std::cmp::min(std::cmp::max(x + 3 * radius_64 / 2, 0), width_wide - 1) as u32)
                    * CHANNEL_CONFIGURATION as u32) as usize;

            let bytes_offset = next_row_y + next_row_x;

            let ur8 = bytes[bytes_offset];
            let ug8 = bytes[bytes_offset + 1];
            let ub8 = bytes[bytes_offset + 2];

            let arr_index = ((x + 2 * radius_64) & 1023) as usize;

            dif_r += ur8.into();
            der_r += dif_r;
            sum_r += der_r;
            unsafe {
                *buffer_r.get_unchecked_mut(arr_index) = ur8.into();
            }

            dif_g += ug8.into();
            der_g += dif_g;
            sum_g += der_g;
            unsafe {
                *buffer_g.get_unchecked_mut(arr_index) = ug8.into();
            }

            dif_b += ub8.into();
            der_b += dif_b;
            sum_b += der_b;
            unsafe {
                *buffer_b.get_unchecked_mut(arr_index) = ub8.into();
            }

            if CHANNEL_CONFIGURATION == 4 {
                let ua8 = bytes[bytes_offset + 3];
                dif_a += ua8.into();
                der_a += dif_a;
                sum_a += der_a;
                unsafe {
                    *buffer_a.get_unchecked_mut(arr_index) = ua8.into();
                }
            }
        }
    }
}

fn fast_gaussian_next_impl<
    T: FromPrimitive + Default + Into<i32> + Send + Sync,
    const CHANNEL_CONFIGURATION: usize,
>(
    bytes: &mut [T],
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    threading_policy: ThreadingPolicy,
) where
    T: std::ops::AddAssign + std::ops::SubAssign + Copy,
    i64: From<T>,
{
    let mut _dispatcher_vertical: fn(
        bytes: &UnsafeSlice<T>,
        stride: u32,
        width: u32,
        height: u32,
        radius: u32,
        start: u32,
        end: u32,
    ) = if BASE_RADIUS_I64_CUTOFF > radius {
        fast_gaussian_next_vertical_pass::<T, i32, CHANNEL_CONFIGURATION>
    } else {
        fast_gaussian_next_vertical_pass::<T, i64, CHANNEL_CONFIGURATION>
    };
    let mut _dispatcher_horizontal: fn(
        bytes: &UnsafeSlice<T>,
        stride: u32,
        width: u32,
        height: u32,
        radius: u32,
        start: u32,
        end: u32,
    ) = if BASE_RADIUS_I64_CUTOFF > radius {
        fast_gaussian_next_horizontal_pass::<T, i32, CHANNEL_CONFIGURATION>
    } else {
        fast_gaussian_next_horizontal_pass::<T, i64, CHANNEL_CONFIGURATION>
    };
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    {
        if BASE_RADIUS_I64_CUTOFF > radius {
            if std::any::type_name::<T>() == "u8" {
                _dispatcher_vertical =
                    fast_gaussian_next_vertical_pass_neon_u8::<T, CHANNEL_CONFIGURATION>;
                _dispatcher_horizontal =
                    fast_gaussian_next_horizontal_pass_neon_u8::<T, CHANNEL_CONFIGURATION>;
            }
        }
    }
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "x86"),
        target_feature = "sse4.1"
    ))]
    {
        if BASE_RADIUS_I64_CUTOFF > radius {
            if std::any::type_name::<T>() == "u8" {
                _dispatcher_vertical =
                    fast_gaussian_next_vertical_pass_sse_u8::<T, CHANNEL_CONFIGURATION>;
                _dispatcher_horizontal =
                    fast_gaussian_next_horizontal_pass_sse_u8::<T, CHANNEL_CONFIGURATION>;
            }
        }
    }
    let thread_count = threading_policy.get_threads_count(width, height) as u32;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count as usize)
        .build()
        .unwrap();

    let unsafe_image = UnsafeSlice::new(bytes);
    pool.scope(|scope| {
        let segment_size = width / thread_count;

        for i in 0..thread_count {
            let start_x = i * segment_size;
            let mut end_x = (i + 1) * segment_size;
            if i == thread_count - 1 {
                end_x = width;
            }
            scope.spawn(move |_| {
                _dispatcher_vertical(&unsafe_image, stride, width, height, radius, start_x, end_x);
            });
        }
    });

    pool.scope(|scope| {
        let segment_size = height / thread_count;

        for i in 0..thread_count {
            let start_y = i * segment_size;
            let mut end_y = (i + 1) * segment_size;
            if i == thread_count - 1 {
                end_y = height;
            }
            scope.spawn(move |_| {
                _dispatcher_horizontal(
                    &unsafe_image,
                    stride,
                    width,
                    height,
                    radius,
                    start_y,
                    end_y,
                );
            });
        }
    });
}

/// Performs gaussian approximation on the image.
///
/// Fast gaussian approximation for u8 image.
/// This is also a VERY fast approximation, however producing more pleasant results than stack blur, or first level of approximation.
/// Radius is limited to 212.
/// Approximation based on binomial filter.
/// O(1) complexity.
///
/// * `stride` - Lane length, default is width * channels_count if not aligned
/// * `width` - Width of the image
/// * `height` - Height of the image
/// * `radius` - Radius is limited to 212
/// * `channels` - Count of channels in the image
///
/// # Panics
/// Panic is stride/width/height/channel configuration do not match provided
pub fn fast_gaussian_next(
    bytes: &mut [u8],
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    channels: FastBlurChannels,
    threading_policy: ThreadingPolicy,
) {
    let acq_radius = std::cmp::min(radius, 280);
    match channels {
        FastBlurChannels::Channels3 => {
            fast_gaussian_next_impl::<u8, 3>(
                bytes,
                stride,
                width,
                height,
                acq_radius,
                threading_policy,
            );
        }
        FastBlurChannels::Channels4 => {
            fast_gaussian_next_impl::<u8, 4>(
                bytes,
                stride,
                width,
                height,
                acq_radius,
                threading_policy,
            );
        }
    }
}

/// Performs gaussian approximation on the image.
///
/// Fast gaussian approximation for u16 image.
/// This is also a VERY fast approximation, however producing more pleasant results than stack blur, or first level of approximation.
/// Radius is limited to 152.
/// Approximation based on binomial filter.
/// O(1) complexity.
///
/// * `stride` - Lane length, default is width * channels_count if not aligned
/// * `width` - Width of the image
/// * `height` - Height of the image
/// * `radius` - Radius more than ~152 is not supported. To use larger radius convert image to f32 and use function for f32
/// * `channels` - Count of channels in the image
///
/// # Panics
/// Panic is stride/width/height/channel configuration do not match provided
pub fn fast_gaussian_next_u16(
    bytes: &mut [u16],
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    channels: FastBlurChannels,
    threading_policy: ThreadingPolicy,
) {
    let acq_radius = std::cmp::min(radius, 152);
    match channels {
        FastBlurChannels::Channels3 => {
            fast_gaussian_next_impl::<u16, 3>(
                bytes,
                stride,
                width,
                height,
                acq_radius,
                threading_policy,
            );
        }
        FastBlurChannels::Channels4 => {
            fast_gaussian_next_impl::<u16, 4>(
                bytes,
                stride,
                width,
                height,
                acq_radius,
                threading_policy,
            );
        }
    }
}

/// Performs gaussian approximation on the image.
///
/// Fast gaussian approximation for u16 image.
/// This is also a VERY fast approximation, however producing more pleasant results than stack blur, or first level of approximation.
/// Approximation based on binomial filter.
/// O(1) complexity.
///
/// * `stride` - Lane length, default is width * channels_count if not aligned
/// * `width` - Width of the image
/// * `height` - Height of the image
/// * `radius` - Almost any radius is supported, in real world radius > 300 is too big for this implementation
/// * `channels` - Count of channels of the image, only 3 and 4 is supported, alpha position, and channels order does not matter
/// * `threading_policy` - Threads usage policy
///
/// # Panics
/// Panic is stride/width/height/channel configuration do not match provided
pub fn fast_gaussian_next_f32(
    bytes: &mut [f32],
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    channels: FastBlurChannels,
    threading_policy: ThreadingPolicy,
) {
    fast_gaussian_next_f32::fast_gaussian_next_impl_f32(
        bytes,
        stride,
        width,
        height,
        radius,
        channels,
        threading_policy,
    );
}

/// Performs gaussian approximation on the image.
///
/// Fast gaussian approximation for f16 image.
/// This is also a VERY fast approximation, however producing more pleasant results than stack blur, or first level of approximation.
/// Approximation based on binomial filter.
/// O(1) complexity.
///
/// * `stride` - Lane length, default is width * channels_count if not aligned
/// * `width` - Width of the image
/// * `height` - Height of the image
/// * `radius` - Almost any radius is supported, in real world radius > 300 is too big for this implementation
/// * `channels` - Count of channels of the image, only 3 and 4 is supported, alpha position, and channels order does not matter
/// * `threading_policy` - Threads usage policy
///
/// # Panics
/// Panic is stride/width/height/channel configuration do not match provided
pub fn fast_gaussian_next_f16(
    bytes: &mut [u16],
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    channels: FastBlurChannels,
    threading_policy: ThreadingPolicy,
) {
    fast_gaussian_next_f16::fast_gaussian_next_impl_f16(
        bytes,
        stride,
        width,
        height,
        radius,
        channels,
        threading_policy,
    );
}

/// Performs gaussian approximation on the image in linear color space
///
/// This is fast approximation that first converts in linear colorspace, performs blur and converts back,
/// operation will be performed in f32 so its cost is significant
/// Approximation based on binomial filter.
/// O(1) complexity.
///
/// * `stride` - Lane length, default is width * channels_count if not aligned
/// * `width` - Width of the image
/// * `height` - Height of the image
/// * `radius` - Almost any reasonable radius is supported
/// * `channels` - Count of channels of the image, only 3 and 4 is supported, alpha position, and channels order does not matter
/// * `threading_policy` - Threads usage policy
/// * `transfer_function` - Transfer function in linear colorspace
///
/// # Panics
/// Panic is stride/width/height/channel configuration do not match provided
pub fn fast_gaussian_next_in_linear(
    in_place: &mut [u8],
    stride: u32,
    width: u32,
    height: u32,
    radius: u32,
    channels: FastBlurChannels,
    threading_policy: ThreadingPolicy,
    transfer_function: TransferFunction,
) {
    let mut linear_data: Vec<f32> =
        vec![0f32; width as usize * height as usize * channels.get_channels()];

    let forward_transformer = match channels {
        FastBlurChannels::Channels3 => rgb_to_linear,
        FastBlurChannels::Channels4 => rgba_to_linear,
    };

    let inverse_transformer = match channels {
        FastBlurChannels::Channels3 => linear_to_rgb,
        FastBlurChannels::Channels4 => linear_to_rgba,
    };

    forward_transformer(
        &in_place,
        stride,
        &mut linear_data,
        width * std::mem::size_of::<f32>() as u32 * channels.get_channels() as u32,
        width,
        height,
        transfer_function,
    );

    fast_gaussian_next_f32::fast_gaussian_next_impl_f32(
        in_place,
        stride,
        width,
        height,
        radius,
        channels,
        threading_policy,
    );

    inverse_transformer(
        &linear_data,
        width * std::mem::size_of::<f32>() as u32 * channels.get_channels() as u32,
        in_place,
        stride,
        width,
        height,
        transfer_function,
    );
}
