// Copyright 2020 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! rutabaga_utils: Utility enums, structs, and implementations needed by the rest of the crate.

use std::ffi::NulError;
use std::fmt;
use std::io::Error as IoError;
use std::num::TryFromIntError;
use std::os::raw::c_char;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::str::Utf8Error;
use std::sync::Arc;

#[cfg(any(target_os = "android", target_os = "linux"))]
use nix::Error as NixError;
use remain::sorted;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
#[cfg(feature = "vulkano")]
use vulkano::device::DeviceCreationError;
#[cfg(feature = "vulkano")]
use vulkano::image::ImageError;
#[cfg(feature = "vulkano")]
use vulkano::instance::InstanceCreationError;
#[cfg(feature = "vulkano")]
use vulkano::memory::DeviceMemoryError;
#[cfg(feature = "vulkano")]
use vulkano::memory::MemoryMapError;
#[cfg(feature = "vulkano")]
use vulkano::VulkanError;
use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;

use crate::rutabaga_os::OwnedDescriptor;

/// Represents a buffer.  `base` contains the address of a buffer, while `len` contains the length
/// of the buffer.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RutabagaIovec {
    pub base: *mut c_void,
    pub len: usize,
}

// SAFETY: trivially safe
unsafe impl Send for RutabagaIovec {}

// SAFETY: trivially safe
unsafe impl Sync for RutabagaIovec {}

/// 3D resource creation parameters.  Also used to create 2D resource.  Constants based on Mesa's
/// (internal) Gallium interface.  Not in the virtio-gpu spec, but should be since dumb resources
/// can't work with gfxstream/virglrenderer without this.
pub const RUTABAGA_PIPE_TEXTURE_2D: u32 = 2;
pub const RUTABAGA_PIPE_BIND_RENDER_TARGET: u32 = 2;
#[repr(C)]
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct ResourceCreate3D {
    pub target: u32,
    pub format: u32,
    pub bind: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub array_size: u32,
    pub last_level: u32,
    pub nr_samples: u32,
    pub flags: u32,
}

/// Blob resource creation parameters.
pub const RUTABAGA_BLOB_MEM_GUEST: u32 = 0x0001;
pub const RUTABAGA_BLOB_MEM_HOST3D: u32 = 0x0002;
pub const RUTABAGA_BLOB_MEM_HOST3D_GUEST: u32 = 0x0003;

pub const RUTABAGA_BLOB_FLAG_USE_MAPPABLE: u32 = 0x0001;
pub const RUTABAGA_BLOB_FLAG_USE_SHAREABLE: u32 = 0x0002;
pub const RUTABAGA_BLOB_FLAG_USE_CROSS_DEVICE: u32 = 0x0004;
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ResourceCreateBlob {
    pub blob_mem: u32,
    pub blob_flags: u32,
    pub blob_id: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RutabagaMapping {
    pub ptr: u64,
    pub size: u64,
}

/// Metadata associated with a swapchain, video or camera image.
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Deserialize, Serialize)]
pub struct Resource3DInfo {
    pub width: u32,
    pub height: u32,
    pub drm_fourcc: u32,
    pub strides: [u32; 4],
    pub offsets: [u32; 4],
    pub modifier: u64,
    /// Whether the buffer can be accessed by the guest CPU.
    pub guest_cpu_mappable: bool,
}

/// A unique identifier for a device.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    FromBytes,
    IntoBytes,
    Immutable,
)]
#[repr(C)]
#[derive(Deserialize, Serialize)]
pub struct DeviceId {
    pub device_uuid: [u8; 16],
    pub driver_uuid: [u8; 16],
}

/// Memory index and physical device id of the associated VkDeviceMemory.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    FromBytes,
    IntoBytes,
    Immutable,
)]
#[repr(C)]
#[derive(Deserialize, Serialize)]
pub struct VulkanInfo {
    pub memory_idx: u32,
    pub device_id: DeviceId,
}

/// Rutabaga context init capset id mask.
pub const RUTABAGA_CONTEXT_INIT_CAPSET_ID_MASK: u32 = 0x00ff;

/// Rutabaga flags for creating fences.
pub const RUTABAGA_FLAG_FENCE: u32 = 1 << 0;
pub const RUTABAGA_FLAG_INFO_RING_IDX: u32 = 1 << 1;
pub const RUTABAGA_FLAG_FENCE_HOST_SHAREABLE: u32 = 1 << 2;

/// Convenience struct for Rutabaga fences
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RutabagaFence {
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub ring_idx: u8,
}

/// Rutabaga debug types
pub const RUTABAGA_DEBUG_ERROR: u32 = 0x01;
pub const RUTABAGA_DEBUG_WARNING: u32 = 0x02;
pub const RUTABAGA_DEBUG_INFO: u32 = 0x03;

/// Convenience struct for debug data
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RutabagaDebug {
    pub debug_type: u32,
    pub message: *const c_char,
}

/// Rutabaga import flags
pub const RUTABAGA_IMPORT_FLAG_3D_INFO: u32 = 1 << 0;
pub const RUTABAGA_IMPORT_FLAG_VULKAN_INFO: u32 = 1 << 1;
pub const RUTABAGA_IMPORT_FLAG_RESOURCE_EXISTS: u32 = 1 << 30;
pub const RUTABAGA_IMPORT_FLAG_PRESERVE_CONTENT: u32 = 1 << 31;

/// Import Data for resource_import
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RutabagaImportData {
    pub flags: u32,
    pub info_3d: Resource3DInfo,
}

// SAFETY:
// This is sketchy, since `message` is a C-string and there's no locking + atomics.  However,
// the current use case is to mirror the C-API.  If the `RutabagaDebugHandler` is used with
// by Rust code, a different struct should be used.
unsafe impl Send for RutabagaDebug {}
// SAFETY:
// This is sketchy, since `message` is a C-string and there's no locking + atomics.  However,
// the current use case is to mirror the C-API.  If the `RutabagaDebugHandler` is used with
// by Rust code, a different struct should be used.
unsafe impl Sync for RutabagaDebug {}

/// Mapped memory caching flags (see virtio_gpu spec)
pub const RUTABAGA_MAP_CACHE_MASK: u32 = 0x0f;
pub const RUTABAGA_MAP_CACHE_CACHED: u32 = 0x01;
pub const RUTABAGA_MAP_CACHE_UNCACHED: u32 = 0x02;
pub const RUTABAGA_MAP_CACHE_WC: u32 = 0x03;
/// Access flags (not in virtio_gpu spec)
pub const RUTABAGA_MAP_ACCESS_MASK: u32 = 0xf0;
pub const RUTABAGA_MAP_ACCESS_READ: u32 = 0x10;
pub const RUTABAGA_MAP_ACCESS_WRITE: u32 = 0x20;
pub const RUTABAGA_MAP_ACCESS_RW: u32 = 0x30;

/// Rutabaga capsets.
pub const RUTABAGA_CAPSET_VIRGL: u32 = 1;
pub const RUTABAGA_CAPSET_VIRGL2: u32 = 2;
pub const RUTABAGA_CAPSET_GFXSTREAM_VULKAN: u32 = 3;
pub const RUTABAGA_CAPSET_VENUS: u32 = 4;
pub const RUTABAGA_CAPSET_CROSS_DOMAIN: u32 = 5;
pub const RUTABAGA_CAPSET_DRM: u32 = 6;
pub const RUTABAGA_CAPSET_GFXSTREAM_MAGMA: u32 = 7;
pub const RUTABAGA_CAPSET_GFXSTREAM_GLES: u32 = 8;
pub const RUTABAGA_CAPSET_GFXSTREAM_COMPOSER: u32 = 9;

/// A list specifying general categories of rutabaga_gfx error.
///
/// This list is intended to grow over time and it is not recommended to exhaustively match against
/// it.
///
/// It is used with the [`RutabagaError`] type.
#[sorted]
#[non_exhaustive]
#[derive(Error, Debug, Clone)]
pub enum RutabagaErrorKind {
    /// Indicates `Rutabaga` was already initialized since only one Rutabaga instance per process
    /// is allowed.
    #[error("attempted to use a rutabaga asset already in use")]
    AlreadyInUse,
    /// Checked Arithmetic error
    #[error("arithmetic failed: {}({}) {op} {}({})", .field1.0, .field1.1, .field2.0, .field2.1)]
    CheckedArithmetic {
        field1: (&'static str, usize),
        field2: (&'static str, usize),
        op: &'static str,
    },
    /// Checked Range error
    #[error("range check failed: {}({}) vs {}({})", .field1.0, .field1.1, .field2.0, .field2.1)]
    CheckedRange {
        field1: (&'static str, usize),
        field2: (&'static str, usize),
    },
    /// An internal Rutabaga component error was returned.
    #[error("rutabaga component failed with error {0}")]
    ComponentError(i32),
    /// Internal error. The caller is not supposed to handle this error.
    #[error("internal error")]
    Internal,
    /// Invalid 2D info
    #[error("invalid 2D info")]
    Invalid2DInfo,
    /// Invalid Capset
    #[error("invalid capset")]
    InvalidCapset,
    /// A command buffer with insufficient space was submitted.
    #[error("invalid command buffer submitted")]
    InvalidCommandBuffer,
    /// A command size was submitted that was invalid.
    #[error("command buffer submitted with invalid size: {0}")]
    InvalidCommandSize(usize),
    /// Invalid RutabagaComponent
    #[error("invalid rutabaga component")]
    InvalidComponent,
    /// Invalid Context ID
    #[error("invalid context id")]
    InvalidContextId,
    /// Invalid cross domain channel
    #[error("invalid cross domain channel")]
    InvalidCrossDomainChannel,
    /// Invalid cross domain item ID
    #[error("invalid cross domain item id")]
    InvalidCrossDomainItemId,
    /// Invalid cross domain item type
    #[error("invalid cross domain item type")]
    InvalidCrossDomainItemType,
    /// Invalid cross domain state
    #[error("invalid cross domain state")]
    InvalidCrossDomainState,
    /// Invalid gralloc backend.
    #[error("invalid gralloc backend")]
    InvalidGrallocBackend,
    /// Invalid gralloc dimensions.
    #[error("invalid gralloc dimensions")]
    InvalidGrallocDimensions,
    /// Invalid gralloc DRM format.
    #[error("invalid gralloc DRM format")]
    InvalidGrallocDrmFormat,
    /// Invalid GPU type.
    #[error("invalid GPU type for gralloc")]
    InvalidGrallocGpuType,
    /// Invalid number of YUV planes.
    #[error("invalid number of YUV planes")]
    InvalidGrallocNumberOfPlanes,
    /// The indicated region of guest memory is invalid.
    #[error("an iovec is outside of guest memory's range")]
    InvalidIovec,
    /// Invalid Resource ID.
    #[error("invalid resource id")]
    InvalidResourceId,
    /// Indicates an error in the RutabagaBuilder.
    #[error("invalid rutabaga build parameters: {0}")]
    InvalidRutabagaBuild(&'static str),
    /// An error with the RutabagaHandle
    #[error("invalid rutabaga handle")]
    InvalidRutabagaHandle,
    /// Invalid Vulkan info
    #[error("invalid vulkan info")]
    InvalidVulkanInfo,
    /// An input/output error occured.
    #[error("an input/output error occur")]
    IoError,
    /// The mapping failed.
    #[error("The mapping failed with library error: {0}")]
    MappingFailed(i32),
    /// Nix crate error.
    #[cfg(any(target_os = "android", target_os = "linux"))]
    #[error("The errno is {0}")]
    NixError(NixError),
    #[error("Nul Error occured {0}")]
    NulError(NulError),
    /// An error with a snapshot.
    #[error("a snapshot error occured: {0}")]
    SnapshotError(String),
    /// Violation of the Rutabaga spec occured.
    #[error("violation of the rutabaga spec: {0}")]
    SpecViolation(&'static str),
    /// An attempted integer conversion failed.
    #[error("int conversion failed: {0}")]
    TryFromIntError(TryFromIntError),
    /// The command is unsupported.
    #[error("the requested function is not implemented")]
    Unsupported,
    /// Utf8 error.
    #[error("an utf8 error occured: {0}")]
    Utf8Error(Utf8Error),
    /// Device creation error
    #[cfg(feature = "vulkano")]
    #[error("vulkano device creation failure {0}")]
    VkDeviceCreationError(DeviceCreationError),
    /// Device memory error
    #[cfg(feature = "vulkano")]
    #[error("vulkano device memory failure {0}")]
    VkDeviceMemoryError(DeviceMemoryError),
    /// General Vulkan error
    #[cfg(feature = "vulkano")]
    #[error("vulkano failure {0}")]
    VkError(VulkanError),
    /// Image creation error
    #[cfg(feature = "vulkano")]
    #[error("vulkano image creation failure {0}")]
    VkImageCreationError(ImageError),
    /// Instance creation error
    #[cfg(feature = "vulkano")]
    #[error("vulkano instance creation failure {0}")]
    VkInstanceCreationError(InstanceCreationError),
    /// Loading error
    #[cfg(feature = "vulkano")]
    #[error("vulkano loading failure")]
    VkLoadingError,
    /// Memory map error
    #[cfg(feature = "vulkano")]
    #[error("vulkano memory map failure {0}")]
    VkMemoryMapError(MemoryMapError),
}

/// An error generated while using this crate.
///
/// Use [`RutabagaError::kind`] to distinguish between different errors.
///
/// # Examples
///
/// To create a [`RutabagaError`], create from an [`anyhow::Error`] or a
/// [`RutabagaErrorKind`].
///
/// ```
/// use rutabaga_gfx::RutabagaError;
/// use rutabaga_gfx::RutabagaErrorKind;
///
/// let error: RutabagaError = anyhow::anyhow!("test error").into();
/// assert!(matches!(error.kind(), &RutabagaErrorKind::Internal));
///
/// let error: RutabagaError = RutabagaErrorKind::AlreadyInUse.into();
/// assert!(matches!(error.kind(), &RutabagaErrorKind::AlreadyInUse));
/// ```
///
/// When creating from an [`anyhow::Error`], if an [`RutabagaErrorKind`] exists in the error chain,
/// the created [`RutabagaError`] will respect that error kind, so feel free to use
/// [`anyhow::Result`] and [`anyhow::Context::context`] in the code base, and only convert the
/// result to [`RutabagaResult`] at the out most public interface.
/// ```
/// use anyhow::Context;
/// use rutabaga_gfx::RutabagaResult;
/// use rutabaga_gfx::RutabagaErrorKind;
///
/// let res = Err::<(), _>(anyhow::anyhow!("test error"))
///     .context("context 1")
///     .context(RutabagaErrorKind::InvalidComponent)
///     .context("context 2");
/// let res: RutabagaResult<()> = res.map_err(|e| e.into());
/// let kind = res.err().map(|e| e.kind().clone());
/// assert!(matches!(kind, Some(RutabagaErrorKind::InvalidComponent)));
///
/// let res = Err::<(), _>(anyhow::anyhow!("test error"))
///     .context("context")
///     .context(RutabagaErrorKind::InvalidComponent);
/// let res: RutabagaResult<()> = res.map_err(|e| e.into());
/// let kind = res.err().map(|e| e.kind().clone());
/// assert!(matches!(kind, Some(RutabagaErrorKind::InvalidComponent)));
///
/// let res = Err::<(), _>(anyhow::anyhow!("test error"))
///     .context(RutabagaErrorKind::InvalidComponent)
///     .context("context");
/// let res: RutabagaResult<()> = res.map_err(|e| e.into());
/// let kind = res.err().map(|e| e.kind().clone());
/// assert!(matches!(kind, Some(RutabagaErrorKind::InvalidComponent)));
/// ```
#[derive(thiserror::Error, Debug)]
#[error("{kind}")]
pub struct RutabagaError {
    kind: RutabagaErrorKind,
    #[source]
    context: Option<anyhow::Error>,
}

impl RutabagaError {
    pub fn kind(&self) -> &RutabagaErrorKind {
        &self.kind
    }
}

impl From<RutabagaErrorKind> for RutabagaError {
    fn from(kind: RutabagaErrorKind) -> Self {
        Self {
            kind,
            context: None,
        }
    }
}

impl From<anyhow::Error> for RutabagaError {
    fn from(value: anyhow::Error) -> Self {
        let kind = value
            .downcast_ref::<RutabagaErrorKind>()
            .unwrap_or(&RutabagaErrorKind::Internal)
            .clone();
        Self {
            kind,
            context: Some(value),
        }
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
impl From<NixError> for RutabagaError {
    fn from(e: NixError) -> RutabagaError {
        RutabagaErrorKind::NixError(e).into()
    }
}

impl From<NulError> for RutabagaError {
    fn from(e: NulError) -> RutabagaError {
        RutabagaErrorKind::NulError(e).into()
    }
}

impl From<IoError> for RutabagaError {
    fn from(e: IoError) -> RutabagaError {
        RutabagaError {
            kind: RutabagaErrorKind::IoError,
            context: Some(anyhow::Error::new(e)),
        }
    }
}

impl From<TryFromIntError> for RutabagaError {
    fn from(e: TryFromIntError) -> RutabagaError {
        RutabagaErrorKind::TryFromIntError(e).into()
    }
}

impl From<Utf8Error> for RutabagaError {
    fn from(e: Utf8Error) -> RutabagaError {
        RutabagaErrorKind::Utf8Error(e).into()
    }
}

/// The result of an operation in this crate.
pub type RutabagaResult<T> = std::result::Result<T, RutabagaError>;

/// Flags for virglrenderer.  Copied from virglrenderer bindings.
const VIRGLRENDERER_USE_EGL: u32 = 1 << 0;
const VIRGLRENDERER_THREAD_SYNC: u32 = 1 << 1;
const VIRGLRENDERER_USE_GLX: u32 = 1 << 2;
const VIRGLRENDERER_USE_SURFACELESS: u32 = 1 << 3;
const VIRGLRENDERER_USE_GLES: u32 = 1 << 4;
const VIRGLRENDERER_USE_EXTERNAL_BLOB: u32 = 1 << 5;
const VIRGLRENDERER_VENUS: u32 = 1 << 6;
const VIRGLRENDERER_NO_VIRGL: u32 = 1 << 7;
const VIRGLRENDERER_USE_ASYNC_FENCE_CB: u32 = 1 << 8;
const VIRGLRENDERER_RENDER_SERVER: u32 = 1 << 9;
const VIRGLRENDERER_DRM: u32 = 1 << 10;

/// virglrenderer flag struct.
#[derive(Copy, Clone)]
pub struct VirglRendererFlags(u32);

impl Default for VirglRendererFlags {
    fn default() -> VirglRendererFlags {
        VirglRendererFlags::new()
            .use_virgl(true)
            .use_venus(false)
            .use_egl(true)
            .use_surfaceless(true)
            .use_gles(true)
            .use_render_server(false)
    }
}

impl From<VirglRendererFlags> for u32 {
    fn from(flags: VirglRendererFlags) -> u32 {
        flags.0
    }
}

impl From<VirglRendererFlags> for i32 {
    fn from(flags: VirglRendererFlags) -> i32 {
        flags.0 as i32
    }
}

impl VirglRendererFlags {
    /// Create new virglrenderer flags.
    pub fn new() -> VirglRendererFlags {
        VirglRendererFlags(0)
    }

    fn set_flag(self, bitmask: u32, set: bool) -> VirglRendererFlags {
        if set {
            VirglRendererFlags(self.0 | bitmask)
        } else {
            VirglRendererFlags(self.0 & (!bitmask))
        }
    }

    /// Enable virgl support
    pub fn use_virgl(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_NO_VIRGL, !v)
    }

    /// Enable venus support
    pub fn use_venus(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_VENUS, v)
    }

    /// Enable drm native context support
    pub fn use_drm(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_DRM, v)
    }

    /// Use EGL for context creation.
    pub fn use_egl(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_USE_EGL, v)
    }

    /// Use a dedicated thread for fence synchronization.
    pub fn use_thread_sync(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_THREAD_SYNC, v)
    }

    /// Use GLX for context creation.
    pub fn use_glx(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_USE_GLX, v)
    }

    /// No surfaces required when creating context.
    pub fn use_surfaceless(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_USE_SURFACELESS, v)
    }

    /// Use GLES drivers.
    pub fn use_gles(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_USE_GLES, v)
    }

    /// Use external memory when creating blob resources.
    pub fn use_external_blob(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_USE_EXTERNAL_BLOB, v)
    }

    /// Retire fence directly from sync thread.
    pub fn use_async_fence_cb(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_USE_ASYNC_FENCE_CB, v)
    }

    pub fn use_render_server(self, v: bool) -> VirglRendererFlags {
        self.set_flag(VIRGLRENDERER_RENDER_SERVER, v)
    }
}

/// Flags for the gfxstream renderer.
const STREAM_RENDERER_FLAGS_USE_EGL: u32 = 1 << 0;
#[allow(dead_code)]
const STREAM_RENDERER_FLAGS_THREAD_SYNC: u32 = 1 << 1;
const STREAM_RENDERER_FLAGS_USE_GLX: u32 = 1 << 2;
const STREAM_RENDERER_FLAGS_USE_SURFACELESS: u32 = 1 << 3;
const STREAM_RENDERER_FLAGS_USE_GLES: u32 = 1 << 4;
const STREAM_RENDERER_FLAGS_USE_VK_BIT: u32 = 1 << 5;
const STREAM_RENDERER_FLAGS_USE_EXTERNAL_BLOB: u32 = 1 << 6;
const STREAM_RENDERER_FLAGS_USE_SYSTEM_BLOB: u32 = 1 << 7;
const STREAM_RENDERER_FLAGS_VULKAN_NATIVE_SWAPCHAIN_BIT: u32 = 1 << 8;

/// gfxstream flag struct.
#[derive(Copy, Clone, Default)]
pub struct GfxstreamFlags(u32);

#[derive(Clone, Debug)]
pub enum RutabagaWsi {
    Surfaceless,
    VulkanSwapchain,
}

impl GfxstreamFlags {
    /// Create new gfxstream flags.
    pub fn new() -> GfxstreamFlags {
        GfxstreamFlags(0)
    }

    fn set_flag(self, bitmask: u32, set: bool) -> GfxstreamFlags {
        if set {
            GfxstreamFlags(self.0 | bitmask)
        } else {
            GfxstreamFlags(self.0 & (!bitmask))
        }
    }

    /// Use EGL for context creation.
    pub fn use_egl(self, v: bool) -> GfxstreamFlags {
        self.set_flag(STREAM_RENDERER_FLAGS_USE_EGL, v)
    }

    /// Use GLX for context creation.
    pub fn use_glx(self, v: bool) -> GfxstreamFlags {
        self.set_flag(STREAM_RENDERER_FLAGS_USE_GLX, v)
    }

    /// No surfaces required when creating context.
    pub fn use_surfaceless(self, v: bool) -> GfxstreamFlags {
        self.set_flag(STREAM_RENDERER_FLAGS_USE_SURFACELESS, v)
    }

    /// Use GLES drivers.
    pub fn use_gles(self, v: bool) -> GfxstreamFlags {
        self.set_flag(STREAM_RENDERER_FLAGS_USE_GLES, v)
    }

    /// Support using Vulkan.
    pub fn use_vulkan(self, v: bool) -> GfxstreamFlags {
        self.set_flag(STREAM_RENDERER_FLAGS_USE_VK_BIT, v)
    }

    /// Use the Vulkan swapchain to draw on the host window.
    pub fn set_wsi(self, v: RutabagaWsi) -> GfxstreamFlags {
        let use_vulkan_swapchain = matches!(v, RutabagaWsi::VulkanSwapchain);
        self.set_flag(
            STREAM_RENDERER_FLAGS_VULKAN_NATIVE_SWAPCHAIN_BIT,
            use_vulkan_swapchain,
        )
    }

    /// Use external blob when creating resources.
    pub fn use_external_blob(self, v: bool) -> GfxstreamFlags {
        self.set_flag(STREAM_RENDERER_FLAGS_USE_EXTERNAL_BLOB, v)
    }

    /// Use system blob when creating resources.
    pub fn use_system_blob(self, v: bool) -> GfxstreamFlags {
        self.set_flag(STREAM_RENDERER_FLAGS_USE_SYSTEM_BLOB, v)
    }
}

impl From<GfxstreamFlags> for u32 {
    fn from(flags: GfxstreamFlags) -> u32 {
        flags.0
    }
}

impl From<GfxstreamFlags> for i32 {
    fn from(flags: GfxstreamFlags) -> i32 {
        flags.0 as i32
    }
}

impl From<GfxstreamFlags> for u64 {
    fn from(flags: GfxstreamFlags) -> u64 {
        flags.0 as u64
    }
}

/// Transfers {to, from} 1D buffers, 2D textures, 3D textures, and cubemaps.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Transfer3D {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
    pub h: u32,
    pub d: u32,
    pub level: u32,
    pub stride: u32,
    pub layer_stride: u32,
    pub offset: u64,
}

impl Transfer3D {
    /// Constructs a 2 dimensional XY box in 3 dimensional space with unit depth and zero
    /// displacement on the Z axis.
    pub fn new_2d(x: u32, y: u32, w: u32, h: u32, offset: u64) -> Transfer3D {
        Transfer3D {
            x,
            y,
            z: 0,
            w,
            h,
            d: 1,
            level: 0,
            stride: 0,
            layer_stride: 0,
            offset,
        }
    }

    /// Returns true if this box represents a volume of zero.
    pub fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0 || self.d == 0
    }
}

/// Rutabaga channel types
pub const RUTABAGA_CHANNEL_TYPE_WAYLAND: u32 = 0x0001;
pub const RUTABAGA_CHANNEL_TYPE_CAMERA: u32 = 0x0002;

/// Information needed to open an OS-specific RutabagaConnection (TBD).  Only Linux hosts are
/// considered at the moment.
#[derive(Clone)]
pub struct RutabagaChannel {
    pub base_channel: PathBuf,
    pub channel_type: u32,
}

/// Enumeration of possible rutabaga components.
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum RutabagaComponentType {
    Rutabaga2D,
    VirglRenderer,
    Gfxstream,
    CrossDomain,
}

impl RutabagaComponentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RutabagaComponentType::CrossDomain => "crossdomain",
            RutabagaComponentType::Gfxstream => "gfxstream",
            RutabagaComponentType::Rutabaga2D => "rutabaga2d",
            RutabagaComponentType::VirglRenderer => "virglrenderer",
        }
    }
}

/// Rutabaga handle types (memory and sync in same namespace)
pub const RUTABAGA_HANDLE_TYPE_MEM_OPAQUE_FD: u32 = 0x0001;
pub const RUTABAGA_HANDLE_TYPE_MEM_DMABUF: u32 = 0x0002;
pub const RUTABAGA_HANDLE_TYPE_MEM_OPAQUE_WIN32: u32 = 0x0003;
pub const RUTABAGA_HANDLE_TYPE_MEM_SHM: u32 = 0x0004;
pub const RUTABAGA_HANDLE_TYPE_MEM_ZIRCON: u32 = 0x0005;

pub const RUTABAGA_HANDLE_TYPE_SIGNAL_OPAQUE_FD: u32 = 0x0010;
pub const RUTABAGA_HANDLE_TYPE_SIGNAL_SYNC_FD: u32 = 0x0020;
pub const RUTABAGA_HANDLE_TYPE_SIGNAL_OPAQUE_WIN32: u32 = 0x0030;
pub const RUTABAGA_HANDLE_TYPE_SIGNAL_ZIRCON: u32 = 0x0040;
pub const RUTABAGA_HANDLE_TYPE_SIGNAL_EVENT_FD: u32 = 0x0050;

pub const RUTABAGA_HANDLE_TYPE_PLATFORM_SCREEN_BUFFER_QNX: u32 = 0x01000000;
pub const RUTABAGA_HANDLE_TYPE_PLATFORM_EGL_NATIVE_PIXMAP: u32 = 0x02000000;

/// Handle to OS-specific memory or synchronization objects.
pub struct RutabagaHandle {
    pub os_handle: OwnedDescriptor,
    pub handle_type: u32,
}

impl fmt::Debug for RutabagaHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle debug").finish()
    }
}

impl RutabagaHandle {
    /// Clones an existing rutabaga handle, by using OS specific mechanisms.
    pub fn try_clone(&self) -> RutabagaResult<RutabagaHandle> {
        let clone = self.os_handle.try_clone().map_err(|e| RutabagaError {
            kind: RutabagaErrorKind::InvalidRutabagaHandle,
            context: Some(anyhow::Error::new(e)),
        })?;
        Ok(RutabagaHandle {
            os_handle: clone,
            handle_type: self.handle_type,
        })
    }
}

#[derive(Clone)]
pub struct RutabagaHandler<S> {
    closure: Arc<dyn Fn(S) + Send + Sync>,
}

impl<S> RutabagaHandler<S>
where
    S: Send + Sync + Clone + 'static,
{
    pub fn new(closure: impl Fn(S) + Send + Sync + 'static) -> RutabagaHandler<S> {
        RutabagaHandler {
            closure: Arc::new(closure),
        }
    }

    pub fn call(&self, data: S) {
        (self.closure)(data)
    }
}

impl<S> fmt::Debug for RutabagaHandler<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Closure debug").finish()
    }
}

pub type RutabagaFenceHandler = RutabagaHandler<RutabagaFence>;

pub type RutabagaDebugHandler = RutabagaHandler<RutabagaDebug>;

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;

    #[test]
    fn error_from_anyhow_should_keep_root_kind() {
        let from_res = Err::<(), _>(RutabagaErrorKind::InvalidComponent)
            .context("context 1")
            .context("context 2");
        let to_res: std::result::Result<_, RutabagaError> = from_res.map_err(|e| e.into());
        let to_kind = to_res.err().map(|e| e.kind().clone());
        assert!(
            matches!(to_kind, Some(RutabagaErrorKind::InvalidComponent)),
            "Expect the error kind to be RutabagaErrorKind::InvalidComponent, but is {:?}",
            to_kind
        );
    }

    #[test]
    fn error_from_anyhow_should_keep_kind_added_in_context() {
        let from_res = Err::<(), _>(anyhow::anyhow!("test error"))
            .context("context 1")
            .context(RutabagaErrorKind::InvalidComponent)
            .context("context 2");
        let to_res: std::result::Result<_, RutabagaError> = from_res.map_err(|e| e.into());
        let to_kind = to_res.err().map(|e| e.kind().clone());
        assert!(
            matches!(to_kind, Some(RutabagaErrorKind::InvalidComponent)),
            "Expect the error kind to be RutabagaErrorKind::InvalidComponent, but is {:?}",
            to_kind
        );
    }

    #[test]
    fn error_from_anyhow_should_keep_top_most_kind() {
        let from_res = Err::<(), _>(anyhow::anyhow!("test error"))
            .context(RutabagaErrorKind::InvalidIovec)
            .context(RutabagaErrorKind::InvalidComponent);
        let to_res: std::result::Result<_, RutabagaError> = from_res.map_err(|e| e.into());
        let to_kind = to_res.err().map(|e| e.kind().clone());
        assert!(
            matches!(to_kind, Some(RutabagaErrorKind::InvalidComponent)),
            "Expect the error kind to be RutabagaErrorKind::InvalidComponent, but is {:?}",
            to_kind
        );
    }

    #[test]
    fn error_from_kind_should_keep_kind() {
        let from_res = Err::<(), _>(RutabagaErrorKind::InvalidComponent);
        let to_res: std::result::Result<_, RutabagaError> = from_res.map_err(|e| e.into());
        let to_kind = to_res.err().map(|e| e.kind().clone());
        assert!(
            matches!(to_kind, Some(RutabagaErrorKind::InvalidComponent)),
            "Expect the error kind to be RutabagaErrorKind::InvalidComponent, but is {:?}",
            to_kind
        );
    }

    #[test]
    fn error_from_arbitrary_anyhow_error_should_be_internal() {
        let from_res = Err::<(), _>(anyhow::anyhow!("test error"));
        let to_res: std::result::Result<_, RutabagaError> = from_res.map_err(|e| e.into());
        let to_kind = to_res.err().map(|e| e.kind().clone());
        assert!(
            matches!(to_kind, Some(RutabagaErrorKind::Internal)),
            "Expect the error kind to be RutabagaErrorKind::Internal, but is {:?}",
            to_kind
        );
    }
}
