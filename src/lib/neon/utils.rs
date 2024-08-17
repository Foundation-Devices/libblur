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

use crate::neon::f16_utils::{
    xreinterpret_f16_u16, xreinterpret_u16_f16, xvld_f16, xvst_f16,
};

#[cfg(target_arch = "aarch64")]
use crate::neon::f16_utils::{
    xvcvt_f16_f32, xvcvt_f32_f16
};

#[cfg(feature = "colorutils-rs")]
use erydanos::vmulq_s64;
use half::f16;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

#[cfg(target_arch = "arm")]
use std::arch::arm::*;

#[inline(always)]
pub(crate) unsafe fn load_u8_s32_fast<const CHANNELS_COUNT: usize>(ptr: *const u8) -> int32x4_t {
    return vreinterpretq_s32_u32(load_u8_u32_fast::<CHANNELS_COUNT>(ptr));
}

#[inline(always)]
pub(crate) unsafe fn load_u8_u32_one(ptr: *const u8) -> uint32x2_t {
    let u_first = u32::from_le_bytes([ptr.read_unaligned(), 0, 0, 0]);
    return vdup_n_u32(u_first);
}

#[inline(always)]
pub(crate) unsafe fn store_f32<const CHANNELS_COUNT: usize>(dst_ptr: *mut f32, regi: float32x4_t) {
    if CHANNELS_COUNT == 4 {
        vst1q_f32(dst_ptr, regi);
    } else if CHANNELS_COUNT == 3 {
        let lo_part = vget_lane_u64::<0>(vreinterpret_u64_f32(vget_low_f32(regi)));
        (dst_ptr as *mut u64).write_unaligned(lo_part);
        dst_ptr.add(2).write_unaligned(vgetq_lane_f32::<2>(regi));
    } else if CHANNELS_COUNT == 2 {
        let lo_part = vget_lane_u64::<0>(vreinterpret_u64_f32(vget_low_f32(regi)));
        (dst_ptr as *mut u64).write_unaligned(lo_part);
    } else {
        dst_ptr.write_unaligned(vgetq_lane_f32::<0>(regi));
    }
}

#[inline(always)]
pub(crate) unsafe fn store_u8_s32<const CHANNELS_COUNT: usize>(dst_ptr: *mut u8, regi: int32x4_t) {
    let s16 = vreinterpret_u16_s16(vqmovn_s32(regi));
    let u16_f = vcombine_u16(s16, s16);
    let v8 = vqmovn_u16(u16_f);
    let pixel_u32 = vget_lane_u32::<0>(vreinterpret_u32_u8(v8));
    if CHANNELS_COUNT == 4 {
        let casted_dst = dst_ptr as *mut u32;
        casted_dst.write_unaligned(pixel_u32);
    } else if CHANNELS_COUNT == 3 {
        let bits = pixel_u32.to_le_bytes();
        let first_byte = u16::from_le_bytes([bits[0], bits[1]]);
        (dst_ptr as *mut u16).write_unaligned(first_byte);
        dst_ptr.add(2).write_unaligned(bits[2]);
    } else if CHANNELS_COUNT == 2 {
        let bits = pixel_u32.to_le_bytes();
        let first_byte = u16::from_le_bytes([bits[0], bits[1]]);
        (dst_ptr as *mut u16).write_unaligned(first_byte);
    } else {
        let bits = pixel_u32.to_le_bytes();
        dst_ptr.write_unaligned(bits[0]);
    }
}

#[inline(always)]
pub(crate) unsafe fn store_u8_u32<const CHANNELS_COUNT: usize>(dst_ptr: *mut u8, regi: uint32x4_t) {
    let s16 = vqmovn_u32(regi);
    let u16_f = vcombine_u16(s16, s16);
    let v8 = vqmovn_u16(u16_f);
    let pixel_u32 = vget_lane_u32::<0>(vreinterpret_u32_u8(v8));
    if CHANNELS_COUNT == 4 {
        let casted_dst = dst_ptr as *mut u32;
        casted_dst.write_unaligned(pixel_u32);
    } else if CHANNELS_COUNT == 3 {
        let bits = pixel_u32.to_le_bytes();
        let first_byte = u16::from_le_bytes([bits[0], bits[1]]);
        (dst_ptr as *mut u16).write_unaligned(first_byte);
        dst_ptr.add(2).write_unaligned(bits[2]);
    } else if CHANNELS_COUNT == 2 {
        let bits = pixel_u32.to_le_bytes();
        let first_byte = u16::from_le_bytes([bits[0], bits[1]]);
        (dst_ptr as *mut u16).write_unaligned(first_byte);
    } else {
        let bits = pixel_u32.to_le_bytes();
        dst_ptr.write_unaligned(bits[0]);
    }
}

#[inline(always)]
pub(crate) unsafe fn load_f32_fast<const CHANNELS_COUNT: usize>(ptr: *const f32) -> float32x4_t {
    if CHANNELS_COUNT == 4 {
        return vld1q_f32(ptr);
    } else if CHANNELS_COUNT == 3 {
        return vld1q_f32(
            [
                ptr.read_unaligned(),
                ptr.add(1).read_unaligned(),
                ptr.add(2).read_unaligned(),
                0f32,
            ]
            .as_ptr(),
        );
    } else if CHANNELS_COUNT == 2 {
        return vld1q_f32(
            [
                ptr.read_unaligned(),
                ptr.add(1).read_unaligned(),
                0f32,
                0f32,
            ]
            .as_ptr(),
        );
    }
    return vld1q_f32([ptr.read_unaligned(), 0f32, 0f32, 0f32].as_ptr());
}

#[inline(always)]
pub(crate) unsafe fn load_u8_f32_fast<const CHANNELS_COUNT: usize>(ptr: *const u8) -> float32x4_t {
    vcvtq_f32_u32(load_u8_u32_fast::<CHANNELS_COUNT>(ptr))
}

#[inline(always)]
pub(crate) unsafe fn load_u8_u32_fast<const CHANNELS_COUNT: usize>(ptr: *const u8) -> uint32x4_t {
    if CHANNELS_COUNT == 3 || CHANNELS_COUNT == 4 {
        let u_first = u32::from_le_bytes([ptr.read_unaligned(), 0, 0, 0]);
        let u_second = u32::from_le_bytes([ptr.add(1).read_unaligned(), 0, 0, 0]);
        let u_third = u32::from_le_bytes([ptr.add(2).read_unaligned(), 0, 0, 0]);
        let u_fourth = match CHANNELS_COUNT {
            4 => u32::from_le_bytes([ptr.add(3).read_unaligned(), 0, 0, 0]),
            _ => 0,
        };
        let store: [u32; 4] = [u_first, u_second, u_third, u_fourth];
        return vld1q_u32(store.as_ptr());
    } else if CHANNELS_COUNT == 2 {
        let u_first = u32::from_le_bytes([ptr.read_unaligned(), 0, 0, 0]);
        let u_second = u32::from_le_bytes([ptr.add(1).read_unaligned(), 0, 0, 0]);
        let store: [u32; 4] = [u_first, u_second, 0, 0];
        return vld1q_u32(store.as_ptr());
    } else {
        let u_first = u32::from_le_bytes([ptr.read_unaligned(), 0, 0, 0]);
        let store: [u32; 4] = [u_first, 0, 0, 0];
        return vld1q_u32(store.as_ptr());
    }
}

#[inline(always)]
pub(crate) unsafe fn load_u8_u64_fast<const CHANNELS_COUNT: usize>(ptr: *const u8) -> uint64x2x2_t {
    let u_first = u64::from_le_bytes([ptr.read_unaligned(), 0, 0, 0, 0, 0, 0, 0]);
    let u_second = u64::from_le_bytes([ptr.add(1).read_unaligned(), 0, 0, 0, 0, 0, 0, 0]);
    let u_third = u64::from_le_bytes([ptr.add(2).read_unaligned(), 0, 0, 0, 0, 0, 0, 0]);
    let u_fourth = match CHANNELS_COUNT {
        4 => u64::from_le_bytes([ptr.add(3).read_unaligned(), 0, 0, 0, 0, 0, 0, 0]),
        _ => 0,
    };
    let store: [u64; 4] = [u_first, u_second, u_third, u_fourth];
    return vld1q_u64_x2(store.as_ptr());
}

#[inline(always)]
pub unsafe fn vaddq_s64x2(ab: int64x2x2_t, cd: int64x2x2_t) -> int64x2x2_t {
    let ux_0 = vaddq_s64(ab.0, cd.0);
    let ux_1 = vaddq_s64(ab.1, cd.1);
    int64x2x2_t(ux_0, ux_1)
}

#[inline(always)]
pub unsafe fn vsubq_s64x2(ab: int64x2x2_t, cd: int64x2x2_t) -> int64x2x2_t {
    let ux_0 = vsubq_s64(ab.0, cd.0);
    let ux_1 = vsubq_s64(ab.1, cd.1);
    int64x2x2_t(ux_0, ux_1)
}

#[inline(always)]
pub(crate) unsafe fn vdupq_n_s64x2(v: i64) -> int64x2x2_t {
    let vl = vdupq_n_s64(v);
    int64x2x2_t(vl, vl)
}

#[inline(always)]
pub(crate) unsafe fn vmulq_u32_f32(a: uint32x4_t, b: float32x4_t) -> uint32x4_t {
    let cvt = vcvtq_f32_u32(a);
    vcvtq_u32_f32(vmulq_f32(cvt, b))
}

#[inline(always)]
pub(crate) unsafe fn vmulq_s32_f32(a: int32x4_t, b: float32x4_t) -> int32x4_t {
    let cvt = vcvtq_f32_s32(a);
    vcvtq_s32_f32(vmulq_f32(cvt, b))
}

#[inline(always)]
pub(crate) unsafe fn vmulq_n_s64x2(x: int64x2x2_t, v: i64) -> int64x2x2_t {
    let vl = vdupq_n_s64(v);
    let lo = vmulq_s64(x.0, vl);
    let hi = vmulq_s64(x.1, vl);
    int64x2x2_t(lo, hi)
}

#[inline(always)]
pub(crate) unsafe fn load_u8_s64x2_fast<const CHANNELS_COUNT: usize>(
    ptr: *const u8,
) -> int64x2x2_t {
    let ux = load_u8_u64_fast::<CHANNELS_COUNT>(ptr);
    let sx_0 = vreinterpretq_s64_u64(ux.0);
    let sx_1 = vreinterpretq_s64_u64(ux.1);
    int64x2x2_t(sx_0, sx_1)
}

#[inline(always)]
pub(crate) unsafe fn load_u8_u16<const CHANNELS_COUNT: usize>(ptr: *const u8) -> uint16x4_t {
    let u_first = u16::from_le_bytes([ptr.read(), 0]);
    let u_second = u16::from_le_bytes([ptr.add(1).read_unaligned(), 0]);
    let u_third = u16::from_le_bytes([ptr.add(2).read_unaligned(), 0]);
    let u_fourth = match CHANNELS_COUNT {
        4 => u16::from_le_bytes([ptr.add(3).read_unaligned(), 0]),
        _ => 0,
    };
    let store: [u16; 4] = [u_first, u_second, u_third, u_fourth];
    let pixel_color = unsafe { vld1_u16(store.as_ptr()) };
    return pixel_color;
}

#[inline(always)]
pub(crate) unsafe fn prefer_vfmaq_f32(
    a: float32x4_t,
    b: float32x4_t,
    c: float32x4_t,
) -> float32x4_t {
    #[cfg(target_arch = "aarch64")]
    {
        return vfmaq_f32(a, b, c);
    }
    #[cfg(target_arch = "arm")]
    {
        return vmlaq_f32(a, b, c);
    }
}

#[inline(always)]
pub(crate) unsafe fn prefer_vfma_f32(
    a: float32x2_t,
    b: float32x2_t,
    c: float32x2_t,
) -> float32x2_t {
    #[cfg(target_arch = "aarch64")]
    {
        return vfma_f32(a, b, c);
    }
    #[cfg(target_arch = "arm")]
    {
        return vmla_f32(a, b, c);
    }
}

#[inline(always)]
pub(crate) unsafe fn vhsumq_f32(a: float32x4_t) -> f32 {
    let va = vadd_f32(vget_low_f32(a), vget_high_f32(a));
    vpadds_f32(va)
}

#[inline(always)]
pub(crate) unsafe fn vsplit_rgb_5(px: float32x4x4_t) -> Float32x5T {
    let first_pixel = vsetq_lane_f32::<3>(0f32, px.0);
    let second_pixel = vsetq_lane_f32::<3>(0f32, vextq_f32::<3>(px.0, px.1));
    let third_pixel = vsetq_lane_f32::<3>(0f32, vextq_f32::<2>(px.1, px.2));
    let four_pixel = vsetq_lane_f32::<3>(0f32, vextq_f32::<1>(px.2, px.3));
    Float32x5T(
        first_pixel,
        second_pixel,
        third_pixel,
        four_pixel,
        vsetq_lane_f32::<3>(0f32, px.3),
    )
}

pub(crate) struct Float32x5T(
    pub float32x4_t,
    pub float32x4_t,
    pub float32x4_t,
    pub float32x4_t,
    pub float32x4_t,
);

#[inline(always)]
#[cfg(target_arch = "aarch64")]
pub(crate) unsafe fn load_f32_f16<const CHANNELS_COUNT: usize>(ptr: *const f16) -> float32x4_t {
    if CHANNELS_COUNT == 4 {
        let cvt = xvld_f16(ptr);
        return xvcvt_f32_f16(cvt);
    } else if CHANNELS_COUNT == 3 {
        let recast = ptr as *const u16;
        let cvt = xreinterpret_f16_u16(vld1_u16(
            [
                recast.read_unaligned(),
                recast.add(1).read_unaligned(),
                recast.add(2).read_unaligned(),
                0,
            ]
            .as_ptr(),
        ));
        return xvcvt_f32_f16(cvt);
    } else if CHANNELS_COUNT == 2 {
        let recast = ptr as *const u16;
        let cvt = xreinterpret_f16_u16(vld1_u16(
            [
                recast.read_unaligned(),
                recast.add(1).read_unaligned(),
                0,
                0,
            ]
            .as_ptr(),
        ));
        return xvcvt_f32_f16(cvt);
    }
    let recast = ptr as *const u16;
    let cvt = xreinterpret_f16_u16(vld1_u16([recast.read_unaligned(), 0, 0, 0].as_ptr()));
    return xvcvt_f32_f16(cvt);
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub(crate) unsafe fn store_f32_f16<const CHANNELS_COUNT: usize>(
    dst_ptr: *mut f16,
    in_regi: float32x4_t,
) {
    let out_regi = xvcvt_f16_f32(in_regi);
    if CHANNELS_COUNT == 4 {
        xvst_f16(dst_ptr, out_regi);
    } else if CHANNELS_COUNT == 3 {
        let casted_out = xreinterpret_u16_f16(out_regi);
        let casted_ptr = dst_ptr as *mut u16;
        let lo_part = vreinterpret_u32_u16(casted_out);
        (casted_ptr as *mut u32).write_unaligned(vget_lane_u32::<0>(lo_part));
        casted_ptr
            .add(2)
            .write_unaligned(vget_lane_u16::<2>(casted_out));
    } else if CHANNELS_COUNT == 2 {
        let casted_out = xreinterpret_u16_f16(out_regi);
        let casted_ptr = dst_ptr as *mut u32;
        let lo_part = vreinterpret_u32_u16(casted_out);
        casted_ptr.write_unaligned(vget_lane_u32::<0>(lo_part));
    } else {
        let casted_out = xreinterpret_u16_f16(out_regi);
        let casted_ptr = dst_ptr as *mut u16;
        casted_ptr.write_unaligned(vget_lane_u16::<0>(casted_out));
    }
}

/// Stores up to 4 values from uint8x8_t
#[inline(always)]
pub(crate) unsafe fn store_u8x8_m4<const CHANNELS_COUNT: usize>(
    dst_ptr: *mut u8,
    in_regi: uint8x8_t,
) {
    let casted_u32 = unsafe { vreinterpret_u32_u8(in_regi) };
    let pixel = unsafe { vget_lane_u32::<0>(casted_u32) };

    if CHANNELS_COUNT == 4 {
        (dst_ptr as *mut u32).write_unaligned(pixel);
    } else if CHANNELS_COUNT == 3 {
        let bits = pixel.to_le_bytes();
        let first_byte = u16::from_le_bytes([bits[0], bits[1]]);
        (dst_ptr as *mut u16).write_unaligned(first_byte);
        dst_ptr.add(2).write_unaligned(bits[2]);
    } else if CHANNELS_COUNT == 2 {
        let bits = pixel.to_le_bytes();
        let first_byte = u16::from_le_bytes([bits[0], bits[1]]);
        (dst_ptr as *mut u16).write_unaligned(first_byte);
    } else {
        let bits = pixel.to_le_bytes();
        dst_ptr.write_unaligned(bits[0]);
    }
}

#[cfg(not(feature = "colorutils-rs"))]
#[inline(always)]
pub unsafe fn vmulq_s64(ab: int64x2_t, cd: int64x2_t) -> int64x2_t {
    vreinterpretq_s64_u64(vmulq_u64(
        vreinterpretq_u64_s64(ab),
        vreinterpretq_u64_s64(cd),
    ))
}

#[cfg(not(feature = "colorutils-rs"))]
#[inline(always)]
/// Multiplies u64 together and takes low part, do not care about overflowing
pub unsafe fn vmulq_u64(ab: uint64x2_t, cd: uint64x2_t) -> uint64x2_t {
    /* ac = (ab & 0xFFFFFFFF) * (cd & 0xFFFFFFFF); */
    let ab_low = vmovn_u64(ab);
    let cd_low = vmovn_u64(cd);
    let ac = vmull_u32(ab_low, cd_low);

    /* b = ab >> 32; */
    let b = vshrq_n_u64::<32>(ab);

    /* bc = b * (cd & 0xFFFFFFFF); */
    let bc = vmull_u32(vmovn_u64(b), vmovn_u64(cd));

    /* d = cd >> 32; */
    let d = vshrq_n_u64::<32>(cd);

    /* ad = (ab & 0xFFFFFFFF) * d; */
    let ad = vmull_u32(vmovn_u64(ab), vmovn_u64(d));

    /* high = bc + ad; */
    let mut high = vaddq_u64(bc, ad);

    /* high <<= 32; */
    high = vshlq_n_u64::<32>(high);

    /* return ac + high; */
    return vaddq_u64(high, ac);
}

extern "platform-intrinsic" {
    pub fn simd_extract<T, U>(x: T, idx: u32) -> U;
}

#[allow(unused)]
macro_rules! simd_extract {
    ($x:expr, $idx:expr $(,)?) => {{
        simd_extract($x, const { $idx })
    }};
    ($x:expr, $idx:expr, $ty:ty $(,)?) => {{
        simd_extract::<_, $ty>($x, const { $idx })
    }};
}

/// Floating-point add pairwise
///
/// [Arm's documentation](https://developer.arm.com/architectures/instruction-sets/intrinsics/vpadds_f32)
#[inline]
#[cfg(all(target_arch = "arm", target_feature = "neon"))]
pub unsafe fn vpadds_f32(a: float32x2_t) -> f32 {
    let a1: f32 = simd_extract!(a, 0);
    let a2: f32 = simd_extract!(a, 1);
    a1 + a2
}
