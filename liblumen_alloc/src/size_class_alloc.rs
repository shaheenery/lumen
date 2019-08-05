use core::alloc::{Alloc, AllocErr, CannotReallocInPlace, Layout};
use core::cmp;
use core::intrinsics::unlikely;
use core::ptr::{self, NonNull};

#[cfg(not(test))]
use alloc::boxed::Box;
#[cfg(not(test))]
use alloc::vec::Vec;

use intrusive_collections::{LinkedListLink, UnsafeRef};
use liblumen_core::alloc::alloc_ref::{self, AsAllocRef};
use liblumen_core::alloc::mmap;
use liblumen_core::alloc::size_classes::{SizeClass, SizeClassIndex};
use liblumen_core::locks::RwLock;

use crate::blocks::ThreadSafeBlockBitSubset;
use crate::carriers::{superalign_down, SUPERALIGNED_CARRIER_SIZE};
use crate::carriers::{SlabCarrier, SlabCarrierList};

pub struct SizeClassAlloc {
    max_size_class: SizeClass,
    size_classes: Box<[SizeClass]>,
    carriers: Box<[RwLock<SlabCarrierList>]>,
}
impl SizeClassAlloc {
    pub fn new(size_classes: &[SizeClass]) -> Self {
        // Initialize to default set of empty slab lists
        let mut carriers = Vec::with_capacity(size_classes.len());
        // Initialize every size class with a single slab carrier
        let mut size_classes = size_classes.to_vec();
        size_classes.sort_by(|a, b| a.to_bytes().cmp(&b.to_bytes()));
        let num_classes = size_classes.len();
        let max_size_class = size_classes[num_classes - 1].clone();
        for size_class in size_classes.iter() {
            let mut list = SlabCarrierList::default();
            let slab = unsafe { Self::create_carrier(*size_class).unwrap() };
            list.push_front(unsafe { UnsafeRef::from_raw(slab) });
            carriers.push(RwLock::new(list));
        }
        let size_classes = size_classes.to_vec();
        Self {
            max_size_class,
            size_classes: size_classes.into_boxed_slice(),
            carriers: carriers.into_boxed_slice(),
        }
    }

    /// Returns the maximum size class in bytes
    #[inline]
    pub fn max_size_class(&self) -> usize {
        self.max_size_class.to_bytes()
    }

    pub unsafe fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        // Ensure allocated region has enough space for carrier header and aligned block
        let size = layout.size();
        if unlikely(size > self.max_size_class.to_bytes()) {
            return Err(AllocErr);
        }
        let (index, size_class) =
            binary_search_next_largest(&self.size_classes, |sc| sc.to_bytes().cmp(&size)).unwrap();
        let carriers = self.carriers[index].read();
        for carrier in carriers.iter() {
            if let Ok(ptr) = carrier.alloc_block() {
                return Ok(ptr);
            }
        }
        drop(carriers);
        // No carriers had availability, create a new carrier, locking
        // the carrier list for this size class while we do so, to avoid
        // other readers from trying to create their own carriers at the
        // same time
        let mut carriers = self.carriers[index].write();
        let carrier_ptr = Self::create_carrier(*size_class)?;
        let carrier = &mut *carrier_ptr;
        // This should never fail, but we only assert that in debug mode
        let result = carrier.alloc_block();
        debug_assert!(result.is_ok());
        carriers.push_front(UnsafeRef::from_raw(carrier_ptr));
        result
    }

    pub unsafe fn reallocate(
        &self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<NonNull<u8>, AllocErr> {
        if unlikely(new_size > self.max_size_class.to_bytes()) {
            return Err(AllocErr);
        }
        let size = layout.size();
        let size_class = self.size_class_for_unchecked(size);
        let new_size_class = self.size_class_for_unchecked(new_size);
        // If the size is in the same size class, we don't have to do anything
        if size_class == new_size_class {
            return Ok(ptr);
        }
        // Otherwise we have to allocate in the new size class,
        // copy to that new block, and deallocate the original block
        let align = layout.align();
        let new_layout = Layout::from_size_align_unchecked(new_size, align);
        let new_ptr = self.allocate(new_layout)?;
        // Copy
        ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr(), cmp::min(size, new_size));
        // Deallocate the original block
        self.deallocate(ptr, layout);
        // Return new block
        Ok(new_ptr)
    }

    #[inline]
    pub unsafe fn realloc_in_place(
        &self,
        _ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<(), CannotReallocInPlace> {
        let size = layout.size();
        let size_class = self.size_class_for_unchecked(size);
        let new_size_class = self.size_class_for_unchecked(new_size);
        // As long as we're in the same size class, success!
        if size_class == new_size_class {
            return Ok(());
        }
        // Otherwise we definitely can't grow, and saying we shrank
        // when we really didn't is kind of a no-no, so we'll just
        // return an error and let the caller decide
        Err(CannotReallocInPlace)
    }

    pub unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        // Locate the owning carrier and deallocate with it
        let raw = ptr.as_ptr();
        // Since the slabs are super-aligned, we can mask off the low
        // bits of the given pointer to find our carrier
        let carrier_ptr = superalign_down(raw as usize)
            as *mut SlabCarrier<LinkedListLink, ThreadSafeBlockBitSubset>;
        let carrier = &mut *carrier_ptr;
        carrier.free_block(raw);
    }

    /// Creates a new, empty slab carrier, unlinked to the allocator
    ///
    /// The carrier is allocated via mmap on supported platforms, or the system
    /// allocator otherwise.
    ///
    /// NOTE: You must make sure to add the carrier to the free list of the
    /// allocator, or it will not be used, and will not be freed
    unsafe fn create_carrier(
        size_class: SizeClass,
    ) -> Result<*mut SlabCarrier<LinkedListLink, ThreadSafeBlockBitSubset>, AllocErr> {
        let size = SUPERALIGNED_CARRIER_SIZE;
        assert!(size_class.to_bytes() < size);
        let carrier_layout = Layout::from_size_align_unchecked(size, size);
        // Allocate raw memory for carrier
        let ptr = mmap::map(carrier_layout)?;
        // Initialize carrier in memory
        let carrier = SlabCarrier::init(ptr.as_ptr(), size, size_class);
        // Return an unsafe ref to this carrier back to the caller
        Ok(carrier)
    }
}

unsafe impl Alloc for SizeClassAlloc {
    #[inline]
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        self.allocate(layout)
    }

    #[inline]
    unsafe fn realloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<NonNull<u8>, AllocErr> {
        self.reallocate(ptr, layout, new_size)
    }

    #[inline]
    unsafe fn grow_in_place(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<(), CannotReallocInPlace> {
        self.realloc_in_place(ptr, layout, new_size)
    }

    #[inline]
    unsafe fn shrink_in_place(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<(), CannotReallocInPlace> {
        self.realloc_in_place(ptr, layout, new_size)
    }

    #[inline]
    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.deallocate(ptr, layout);
    }
}

impl<'a> AsAllocRef<'a> for SizeClassAlloc {
    type Handle = alloc_ref::Handle<'a, Self>;

    #[inline]
    fn as_alloc_ref(&self) -> Self::Handle {
        alloc_ref::Handle::new(self)
    }
}

impl Drop for SizeClassAlloc {
    fn drop(&mut self) {
        // Drop slab carriers
        let slab_size = SUPERALIGNED_CARRIER_SIZE;
        let slab_layout = unsafe { Layout::from_size_align_unchecked(slab_size, slab_size) };

        for carrier in self.carriers.iter() {
            // Lock the class list while we free the slabs in it
            let mut list = carrier.write();
            // Collect the pointers/layouts for all slabs allocated in this class
            let mut slabs = list
                .iter()
                .map(|slab| (slab as *const _ as *mut _, slab_layout.clone()))
                .collect::<Vec<_>>();

            // Clear the list without dropping the elements, since we're handling that
            list.fast_clear();

            // Free the memory for all the slabs
            for (ptr, layout) in slabs.drain(..) {
                unsafe { mmap::unmap(ptr, layout) }
            }
        }
    }
}

unsafe impl Sync for SizeClassAlloc {}
unsafe impl Send for SizeClassAlloc {}

/// Represents a type which can map allocation sizes to size class sizes
impl SizeClassIndex for SizeClassAlloc {
    /// Given a SizeClass returned by `size_class_for`, this returns the
    /// position of the size class in the index
    #[inline]
    fn index_for(&self, size_class: SizeClass) -> usize {
        match binary_search_next_largest(&self.size_classes, |sc| sc.cmp(&size_class)) {
            None => usize::max_value(),
            Some((index, _)) => index,
        }
    }

    /// Maps a requested allocation size to the nearest size class size,
    /// if a size class is available to fill the request, otherwise returns None
    #[inline]
    fn size_class_for(&self, request_size: usize) -> Option<SizeClass> {
        match binary_search_next_largest(&self.size_classes, |sc| sc.to_bytes().cmp(&request_size))
        {
            None => None,
            Some((_, found)) => Some(found.clone()),
        }
    }

    /// Same as size_class for, but optimized when the request size is known to be valid
    #[inline]
    unsafe fn size_class_for_unchecked(&self, request_size: usize) -> SizeClass {
        self.size_class_for(request_size).unwrap()
    }
}

/// This function searches a slice for the element that is the
/// closest match to the provided value, and at least as large,
/// and returns `(index, &value)`
#[inline]
fn binary_search_next_largest<'a, T, F>(s: &'a [T], mut f: F) -> Option<(usize, &'a T)>
where
    F: FnMut(&'a T) -> cmp::Ordering,
{
    use cmp::Ordering;

    let mut size = s.len();
    if size == 0 {
        return None;
    }
    let mut base = 0;
    let mut last_largest = None;
    while size > 1 {
        let half = size / 2;
        let mid = base + half;
        let elem = unsafe { s.get_unchecked(mid) };
        match f(elem) {
            Ordering::Equal => return Some((mid, elem)),
            Ordering::Greater => {
                last_largest = Some((mid, elem));
                size -= half;
            }
            Ordering::Less => {
                base = mid;
                size -= half;
            }
        }
    }
    let elem = unsafe { s.get_unchecked(base) };
    match f(elem) {
        Ordering::Equal => Some((base, elem)),
        Ordering::Less => last_largest,
        Ordering::Greater => Some((base, elem)),
    }
}
