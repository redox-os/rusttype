#[cfg(feature = "no_std")]
use core::intrinsics;

#[cfg(not(feature = "no_std"))]
pub fn sqrt(n: f32) -> f32 {
    n.sqrt()
}

#[cfg(feature = "no_std")]
pub fn sqrt(n: f32) -> f32 {
    unsafe { intrinsics::sqrtf32(n) }
}
