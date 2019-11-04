#[cfg(feature = "no_std")]
use core::intrinsics;

#[cfg(not(feature = "no_std"))]
pub fn max(a: f32, b: f32) -> f32 {
    a.max(b)
}

#[cfg(feature = "no_std")]
pub fn max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

#[cfg(not(feature = "no_std"))]
pub fn min(a: f32, b: f32) -> f32 {
    a.min(b)
}

#[cfg(feature = "no_std")]
pub fn min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

#[cfg(not(feature = "no_std"))]
pub fn sqrt(n: f32) -> f32 {
    n.sqrt()
}

#[cfg(feature = "no_std")]
pub fn sqrt(n: f32) -> f32 {
    unsafe { intrinsics::sqrtf32(n) }
}
