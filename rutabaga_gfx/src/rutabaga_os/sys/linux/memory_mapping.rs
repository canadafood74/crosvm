// Copyright 2023 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::num::NonZeroUsize;
use std::os::fd::AsFd;
use std::ptr::NonNull;

use libc::c_void;
use nix::sys::mman::mmap;
use nix::sys::mman::munmap;
use nix::sys::mman::MapFlags;
use nix::sys::mman::ProtFlags;

use crate::rutabaga_os::OwnedDescriptor;
use crate::rutabaga_utils::RutabagaErrorKind;
use crate::rutabaga_utils::RutabagaResult;
use crate::rutabaga_utils::RUTABAGA_MAP_ACCESS_MASK;
use crate::rutabaga_utils::RUTABAGA_MAP_ACCESS_READ;
use crate::rutabaga_utils::RUTABAGA_MAP_ACCESS_RW;
use crate::rutabaga_utils::RUTABAGA_MAP_ACCESS_WRITE;

/// Wraps an anonymous shared memory mapping in the current process. Provides
/// RAII semantics including munmap when no longer needed.
#[derive(Debug)]
pub struct MemoryMapping {
    pub addr: NonNull<c_void>,
    pub size: usize,
}

impl Drop for MemoryMapping {
    fn drop(&mut self) {
        // SAFETY:
        // This is safe because we mmap the area at addr ourselves, and nobody
        // else is holding a reference to it.
        unsafe {
            munmap(self.addr, self.size).unwrap();
        }
    }
}

impl MemoryMapping {
    pub fn from_safe_descriptor(
        descriptor: OwnedDescriptor,
        size: usize,
        map_info: u32,
    ) -> RutabagaResult<MemoryMapping> {
        let non_zero_opt = NonZeroUsize::new(size);
        let prot = match map_info & RUTABAGA_MAP_ACCESS_MASK {
            RUTABAGA_MAP_ACCESS_READ => ProtFlags::PROT_READ,
            RUTABAGA_MAP_ACCESS_WRITE => ProtFlags::PROT_WRITE,
            RUTABAGA_MAP_ACCESS_RW => ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            _ => return Err(RutabagaErrorKind::SpecViolation("incorrect access flags").into()),
        };

        if let Some(non_zero_size) = non_zero_opt {
            // TODO(b/315870313): Add safety comment
            #[allow(clippy::undocumented_unsafe_blocks)]
            let addr = unsafe {
                mmap(
                    None,
                    non_zero_size,
                    prot,
                    MapFlags::MAP_SHARED,
                    descriptor.as_fd(),
                    0,
                )?
            };
            Ok(MemoryMapping { addr, size })
        } else {
            Err(RutabagaErrorKind::SpecViolation("zero size mapping").into())
        }
    }
}
