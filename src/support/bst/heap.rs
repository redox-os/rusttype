use std::mem;

#[inline]
fn ptr_from_vec<T>(mut buf: Vec<T>) -> *mut u8 {
    let ptr = buf.as_mut_ptr() as *mut u8;
    mem::forget(buf);
    ptr
}

#[inline]
pub unsafe fn allocate<Align>(size: usize) -> *mut u8 {
    ptr_from_vec(Vec::<Align>::with_capacity(size / mem::size_of::<Align>()))
}

#[inline]
pub unsafe fn deallocate<Align>(p: *mut u8, old_size: usize) {
    Vec::<Align>::from_raw_parts(p as *mut Align, 0, old_size / mem::size_of::<Align>());
}
// #[inline]
// pub unsafe fn reallocate<T>(p: *mut u8, old_size: usize, size: usize) -> *mut u8 {
// unimplemented!()
// }
