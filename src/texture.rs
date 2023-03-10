use std::{
    fmt,
    fmt::Formatter,
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Deref, DerefMut},
    path::Path,
};
use std::sync::atomic::{AtomicBool, Ordering};

use bytemuck::{Pod, Zeroable};
use duplicate::duplicate_item as duplicate;
use eyre::{Context, Result};
use gl::types::*;
use glam::{UVec2, UVec3};
use num_derive::FromPrimitive;

use crate::{
    base::{
        GlType,
        resource::{Resource, ResourceExt},
    },
    program::Uniform,
    utils::gl_error_guard,
};

pub trait TextureFormat {
    type Subpixel: GlType + Pod;
    const COUNT: usize;
    const FORMAT: GLenum;
    const TYPE: GLenum;
    const NORMALIZED: bool;
}

#[duplicate(
rust_t      internal_format     format;
[u8]        [gl::R8]            [gl::RED];
[i8]        [gl::R8I]           [gl::RED_INTEGER];
[u16]       [gl::R16]           [gl::RED];
[i16]       [gl::R16I]          [gl::RED_INTEGER];
[u32]       [gl::R32UI]         [gl::RED];
[i32]       [gl::R32I]          [gl::RED_INTEGER];
[f32]       [gl::R32F]          [gl::RED];
)]
impl TextureFormat for rust_t {
    type Subpixel = Self;
    const COUNT: usize = 1;
    const FORMAT: GLenum = format;
    const TYPE: GLenum = internal_format;
    const NORMALIZED: bool = false;
}

#[duplicate(
rust_t      internal_format     format;
[u8]        [gl::RG8]           [gl::RG];
[i8]        [gl::RG8I]          [gl::RG_INTEGER];
[u16]       [gl::RG16]          [gl::RG];
[i16]       [gl::RG16I]         [gl::RG_INTEGER];
[u32]       [gl::RG32UI]        [gl::RG];
[i32]       [gl::RG32I]         [gl::RG_INTEGER];
[f32]       [gl::RG32F]         [gl::RG];
)]
impl TextureFormat for [rust_t; 2] {
    type Subpixel = rust_t;
    const COUNT: usize = 2;
    const FORMAT: GLenum = format;
    const TYPE: GLenum = internal_format;
    const NORMALIZED: bool = false;
}

#[duplicate(
rust_t      internal_format     format;
[u8]        [gl::RGB8]          [gl::RGB];
[i8]        [gl::RGB8I]         [gl::RGB_INTEGER];
[u16]       [gl::RGB16]         [gl::RGB];
[i16]       [gl::RGB16I]        [gl::RGB_INTEGER];
[u32]       [gl::RGB32UI]       [gl::RGB];
[i32]       [gl::RGB32I]        [gl::RGB_INTEGER];
[f32]       [gl::RGB32F]        [gl::RGB];
)]
impl TextureFormat for [rust_t; 3] {
    type Subpixel = rust_t;
    const COUNT: usize = 3;
    const FORMAT: GLenum = format;
    const TYPE: GLenum = internal_format;
    const NORMALIZED: bool = false;
}

#[duplicate(
rust_t      internal_format       format;
[u8]        [gl::RGBA8]           [gl::RGBA];
[i8]        [gl::RGBA8I]          [gl::RGBA_INTEGER];
[u16]       [gl::RGBA16]          [gl::RGBA];
[i16]       [gl::RGBA16I]         [gl::RGBA_INTEGER];
[u32]       [gl::RGBA32UI]        [gl::RGBA];
[i32]       [gl::RGBA32I]         [gl::RGBA_INTEGER];
[f32]       [gl::RGBA32F]         [gl::RGBA];
)]
impl TextureFormat for [rust_t; 4] {
    type Subpixel = rust_t;
    const COUNT: usize = 4;
    const FORMAT: GLenum = format;
    const TYPE: GLenum = internal_format;
    const NORMALIZED: bool = false;
}

pub trait AsTextureFormat {
    type TextureFormat: TextureFormat;
}

impl<F: TextureFormat> AsTextureFormat for F {
    type TextureFormat = F;
}

#[cfg(feature = "img")]
impl<F: TextureFormat> AsTextureFormat for image::Luma<F> {
    type TextureFormat = F;
}

#[cfg(feature = "img")]
impl<F> AsTextureFormat for image::LumaA<F>
where
    [F; 2]: TextureFormat,
{
    type TextureFormat = [F; 2];
}

#[cfg(feature = "img")]
impl<F> AsTextureFormat for image::Rgb<F>
where
    [F; 3]: TextureFormat,
{
    type TextureFormat = [F; 3];
}

#[cfg(feature = "img")]
impl<F> AsTextureFormat for image::Rgba<F>
where
    [F; 4]: TextureFormat,
{
    type TextureFormat = [F; 4];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Normalized<F>(PhantomData<F>);

impl<F: TextureFormat> TextureFormat for Normalized<F> {
    type Subpixel = F::Subpixel;
    const COUNT: usize = F::COUNT;
    const FORMAT: GLenum = F::FORMAT;
    const TYPE: GLenum = F::TYPE;
    const NORMALIZED: bool = true;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepthStencil<F, S>(PhantomData<(F, S)>);

impl TextureFormat for DepthStencil<f32, ()> {
    type Subpixel = f32;
    const COUNT: usize = 1;
    const FORMAT: GLenum = gl::DEPTH_COMPONENT;
    const TYPE: GLenum = gl::DEPTH_COMPONENT;
    const NORMALIZED: bool = false;
}

impl TextureFormat for DepthStencil<f32, u8> {
    type Subpixel = f32;
    const COUNT: usize = 1;
    const FORMAT: GLenum = gl::DEPTH32F_STENCIL8;
    const TYPE: GLenum = gl::DEPTH_STENCIL;
    const NORMALIZED: bool = false;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureId {
    id: NonZeroU32,
    pub target: TextureTarget,
}

impl fmt::Display for TextureId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id.get())
    }
}

impl TextureId {
    pub fn new(id: u32, target: TextureTarget) -> Option<Self> {
        Some(Self {
            id: NonZeroU32::new(id)?,
            target,
        })
    }
}

impl Deref for TextureId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum Dimension {
    D1 = gl::TEXTURE_1D,
    D1Array = gl::TEXTURE_1D_ARRAY,
    D2 = gl::TEXTURE_2D,
    D2Array = gl::TEXTURE_2D_ARRAY,
    D3 = gl::TEXTURE_3D,
}

impl Dimension {
    pub fn num_dimension(&self) -> u8 {
        match self {
            Self::D1 => 1,
            Self::D2 => 2,
            Self::D3 => 3,
            Self::D1Array => 11,
            Self::D2Array => 12,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextureTarget {
    pub dim: Dimension,
    pub samples: NonZeroU32,
}

impl PartialEq for TextureTarget {
    fn eq(&self, other: &Self) -> bool {
        self.gl_target().eq(&other.gl_target())
    }
}

impl Eq for TextureTarget {}

impl TextureTarget {
    pub fn is_multisample(&self) -> bool {
        self.samples.get() > 1
    }

    pub fn gl_target(&self) -> GLenum {
        use Dimension::*;

        match (self.dim, self.is_multisample()) {
            (D1, _) => gl::TEXTURE_1D,
            (D1Array, _) => gl::TEXTURE_1D_ARRAY,
            (D2, false) => gl::TEXTURE_2D,
            (D2, true) => gl::TEXTURE_2D_MULTISAMPLE,
            (D2Array, false) => gl::TEXTURE_2D_ARRAY,
            (D2Array, true) => gl::TEXTURE_2D_MULTISAMPLE_ARRAY,
            (D3, _) => gl::TEXTURE_3D,
        }
    }

    pub fn binding_const(&self) -> GLenum {
        use Dimension::*;
        match (self.dim, self.is_multisample()) {
            (D1, _) => gl::TEXTURE_BINDING_1D,
            (D1Array, _) => gl::TEXTURE_BINDING_1D_ARRAY,
            (D2, false) => gl::TEXTURE_BINDING_2D,
            (D2, true) => gl::TEXTURE_BINDING_2D_MULTISAMPLE_ARRAY,
            (D2Array, false) => gl::TEXTURE_BINDING_2D_ARRAY,
            (D2Array, true) => gl::TEXTURE_BINDING_2D_MULTISAMPLE_ARRAY,
            (D3, _) => gl::TEXTURE_BINDING_3D,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum TextureWrap {
    Repeat = gl::REPEAT,
    MirroredRepeat = gl::MIRRORED_REPEAT,
    ClampEdge = gl::CLAMP_TO_EDGE,
    ClampBorder = gl::CLAMP_TO_BORDER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum SampleMode {
    Nearest = gl::NEAREST,
    Linear = gl::LINEAR,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct TextureUnit(u32);

impl GlType for TextureUnit {
    const GL_TYPE: GLenum = gl::UNSIGNED_INT;

    const NUM_COMPONENTS: usize = 1;

    const NORMALIZED: bool = false;

    const STRIDE: usize = std::mem::size_of::<Self>();
}

impl Uniform for TextureUnit {
    unsafe fn write_uniform(&self, location: GLint) {
        tracing::trace!("glUniform1i(<location>, GL_TEXTURE{})", self.0);
        gl::Uniform1i(location, self.0 as _);
    }
}

// TODO: Refactor texture implementation into a "generic texture" vs. "Texture2D" specializations
#[derive(Debug)]
pub struct Texture<F> {
    __fmt: PhantomData<*mut F>,
    width: NonZeroU32,
    height: NonZeroU32,
    depth: NonZeroU32,
    id: TextureId,
    has_mipmaps: AtomicBool,
}

impl<'a, F: 'a> Resource<'a> for Texture<F> {
    type Id = TextureId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn current() -> Option<Self::Id> {
        None
    }

    fn bind(&self) {
        unsafe { gl::BindTexture(self.id.target.gl_target(), self.id.get()) }
    }

    fn unbind(&self) {
        unsafe { gl::BindTexture(self.id.target.gl_target(), 0) }
    }
}

impl<F> Texture<F> {
    pub fn new(width: NonZeroU32, height: NonZeroU32, depth: NonZeroU32, dim: Dimension) -> Self {
        Self::new_multisampled(width, height, depth, dim, NonZeroU32::new(1).unwrap())
    }

    pub fn new_multisampled(
        width: NonZeroU32,
        height: NonZeroU32,
        depth: NonZeroU32,
        dim: Dimension,
        samples: NonZeroU32,
    ) -> Self {
        let mut id = 0;
        unsafe { gl::GenTextures(1, &mut id) }
        Self {
            __fmt: PhantomData,
            width,
            height,
            depth,
            has_mipmaps: AtomicBool::new(false),
            id: TextureId::new(id, TextureTarget { dim, samples }).unwrap(),
        }
    }

    pub fn size(&self) -> (NonZeroU32, NonZeroU32, NonZeroU32) {
        (self.width, self.height, self.depth)
    }

    pub fn size_vec(&self) -> UVec3 {
        UVec3::new(self.width.get(), self.height.get(), self.depth.get())
    }

    /// Returns the texture unit uniform that binds a sampler of this texture into a shader program.
    /// This also binds the texture.
    pub fn as_uniform(&self, unit: u32) -> Result<TextureUnit> {
        eyre::ensure!(
            unit < gl::MAX_COMBINED_TEXTURE_IMAGE_UNITS,
            format!(
                "Trying to activate unit {} which is above the maximum supported of {}",
                unit,
                gl::MAX_COMBINED_TEXTURE_IMAGE_UNITS
            )
        );
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + unit);
        }
        self.bind();
        Ok(TextureUnit(unit))
    }

    pub fn dimension(&self) -> Dimension {
        self.id.target.dim
    }

    pub fn samples(&self) -> u32 {
        self.id.target.samples.get()
    }

    pub fn is_multisample(&self) -> bool {
        self.id.target.is_multisample()
    }

    pub fn id(&self) -> TextureId {
        self.id
    }

    pub fn mipmap_size(&self, mipmap: usize) -> Result<(NonZeroU32, NonZeroU32)> {
        self.bind();
        let mut width = 0;
        let mut height = 0;
        gl_error_guard(|| unsafe {
            gl::GetTexLevelParameteriv(
                self.id.target.gl_target(),
                mipmap as _,
                gl::TEXTURE_WIDTH,
                &mut width,
            );
            gl::GetTexLevelParameteriv(
                self.id.target.gl_target(),
                mipmap as _,
                gl::TEXTURE_HEIGHT,
                &mut height,
            );
        })?;
        let Some(width) = NonZeroU32::new(width as _) else { eyre::bail!("Zero texture size");};
        let Some(height) = NonZeroU32::new(height as _) else { eyre::bail!("Zero texture size");};
        Ok((width, height))
    }

    pub fn num_mipmaps(&self) -> usize {
        let n = if self.has_mipmaps.load(Ordering::Relaxed) {
            f32::log2(self.width.max(self.height).max(self.depth).get() as _).floor() as usize
        } else {
            0
        };
        1 + n
    }

    pub(crate) fn raw_id(&self) -> u32 {
        self.id.get()
    }
}

impl<F: TextureFormat> Texture<F> {
    pub fn from_2d_pixels(width: NonZeroU32, data: &[F::Subpixel]) -> Result<Self> {
        let Some(len) = NonZeroU32::new(data.len() as _) else {
            eyre::bail!("Cannot create empty texture");
        };
        eyre::ensure!(
            (data.len() / F::COUNT) % width.get() as usize == 0,
            "Data slice must be a rectangular array of pixels"
        );
        let height = NonZeroU32::new(len.get() / F::COUNT as u32 / width.get()).unwrap();
        let this = Self::new(
            width as _,
            height as _,
            NonZeroU32::new(1).unwrap(),
            Dimension::D2,
        );
        this.set_data(data)?;
        Ok(this)
    }

    pub fn read_pixel(&self, pos: UVec2) -> Result<F::Subpixel> {
        let mut data = vec![0u8; F::Subpixel::STRIDE];
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::ReadPixels(
                    pos.x as _,
                    pos.y as _,
                    1,
                    1,
                    F::FORMAT,
                    F::Subpixel::GL_TYPE,
                    data.as_mut_ptr().cast(),
                )
            })
        })?;
        let data = bytemuck::cast_vec(data);
        Ok(data[0])
    }

    pub fn download(&self, level: usize) -> Result<Vec<F::Subpixel>> {
        eyre::ensure!(
            level < self.num_mipmaps(),
            "Cannot get level higher than the number of mipmaps in this texture"
        );
        self.bind();

        let (width, height) = self.mipmap_size(level)?;
        let size = width.get() as usize
            * height.get() as usize
            * F::COUNT
            * std::mem::size_of::<F::Subpixel>();

        let mut data = vec![F::Subpixel::zeroed(); size];
        gl_error_guard(|| unsafe {
            gl::GetTexImage(
                self.id.target.gl_target(),
                level as _,
                F::FORMAT,
                F::Subpixel::GL_TYPE,
                // (std::mem::size_of::<F::Subpixel>() * size) as _,
                data.as_mut_ptr().cast(),
            );
        })?;
        Ok(data)
    }

    #[cfg(feature = "img")]
    pub fn download_image<P: 'static + image::Pixel<Subpixel = F::Subpixel>>(
        &self,
        level: usize,
    ) -> Result<image::ImageBuffer<P, Vec<P::Subpixel>>> {
        let (width, height) = self.mipmap_size(level)?;
        let data = self.download(level)?;
        let mut image = image::ImageBuffer::from_vec(width.get(), height.get(), data).unwrap();
        image::imageops::flip_vertical_in_place(&mut image);
        Ok(image)
    }

    #[cfg(feature = "img")]
    pub fn from_image<
        P: image::Pixel<Subpixel = F::Subpixel> + AsTextureFormat<TextureFormat = F>,
        C: Deref<Target = [P::Subpixel]> + DerefMut,
    >(
        mut image: image::ImageBuffer<P, C>,
    ) -> Result<Self>
    where
        P::Subpixel: GlType + Pod,
    {
        image::imageops::flip_vertical_in_place(&mut image);
        Self::from_2d_pixels(image.width().try_into()?, image.as_raw())
    }

    // TODO: Support Non-2D textures
    #[tracing::instrument(skip_all)]
    pub fn reserve_memory(&self) -> Result<()> {
        eyre::ensure!(
            self.id.target.dim == Dimension::D2,
            "Non-2D texture not supported at the moment"
        );
        tracing::trace!(
            "glTexImage2D(<target for dimension {:?}>, 0, <INTERNAL_FORMAT {:x}>, {}, {}, 0, ..., NULL)",
            self.id.target.dim,
            F::TYPE,
            self.width,
            self.height
        );
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::TexImage2D(
                    self.id.target.gl_target(),
                    0,
                    F::TYPE as _,
                    self.width.get() as _,
                    self.height.get() as _,
                    0,
                    F::FORMAT,
                    F::Subpixel::GL_TYPE,
                    std::ptr::null(),
                )
            })
        })
    }

    pub fn set_data(&self, data: &[F::Subpixel]) -> Result<()> {
        let Some(len) = NonZeroU32::new(data.len() as _) else { eyre::bail!("Cannot set empty data"); };
        eyre::ensure!(
            // self.width * self.height * self.depth * F::COUNT as u32
            self.width
                .checked_mul(self.height)
                .unwrap()
                .checked_mul(NonZeroU32::new(F::COUNT as _).unwrap())
                .unwrap()
                == len,
            "Data length has to match the extents of the texture"
        );

        let bytes: &[u8] = bytemuck::cast_slice(data);
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                use Dimension::*;
                match (self.id.target.dim, self.id.target.is_multisample()) {
                    (D2, false) => gl::TexImage2D(
                        self.id.target.gl_target(),
                        0,
                        F::TYPE as _,
                        self.width.get() as _,
                        self.height.get() as _,
                        0,
                        F::FORMAT,
                        F::Subpixel::GL_TYPE,
                        bytes.as_ptr() as *const _,
                    ),
                    (D2, true) => gl::TexImage2DMultisample(
                        self.id.target.gl_target(),
                        self.id.target.samples.get() as _,
                        F::TYPE as _,
                        self.width.get() as _,
                        self.height.get() as _,
                        gl::TRUE,
                    ),
                    _ => todo!(),
                }
            })
        })?;
        self.generate_mipmaps()?;
        Ok(())
    }

    pub fn set_sub_data_2d(
        &self,
        level: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        data: &[F::Subpixel],
    ) -> Result<()> {
        eyre::ensure!(x >= 0, "Sub data rectangle exceeds texture bounds");
        eyre::ensure!(y >= 0, "Sub data rectangle exceeds texture bounds");
        eyre::ensure!(
            x + w < self.width.get() as _,
            "Sub data rectangle exceeds texture bounds"
        );
        eyre::ensure!(
            y + h < self.height.get() as _,
            "Sub data rectangle exceeds texture bounds"
        );
        eyre::ensure!(
            level < self.num_mipmaps(),
            "Sub data rectangle exceeds texture bounds"
        );

        let bytes: &[u8] = bytemuck::cast_slice(data);
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                match (self.id.target.dim, self.id.target.is_multisample()) {
                    (Dimension::D2, false) => gl::TexSubImage2D(
                        self.id.target.gl_target(),
                        level as _,
                        x,
                        y,
                        w,
                        h,
                        F::FORMAT,
                        F::Subpixel::GL_TYPE,
                        bytes.as_ptr().cast(),
                    ),
                    _ => todo!(),
                }
            })
        })
    }

    pub fn generate_mipmaps(&self) -> Result<()> {
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::GenerateMipmap(self.id.target.gl_target());
            })
        })?;
        self.has_mipmaps.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn clear_resize(
        &mut self,
        width: NonZeroU32,
        height: NonZeroU32,
        depth: NonZeroU32,
    ) -> Result<()> {
        self.width = width;
        self.height = height;
        self.depth = depth;
        self.reserve_memory()
            .context("Failed to reserve memory following clear")?;
        self.generate_mipmaps()
            .context("Cannot generate texture mipmaps")?;
        Ok(())
    }

    pub fn wrap_s(&self, wrap: TextureWrap) -> Result<()> {
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::TexParameteri(self.id.target.gl_target(), gl::TEXTURE_WRAP_S, wrap as _);
            })
        })
    }

    pub fn wrap_t(&self, wrap: TextureWrap) -> Result<()> {
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::TexParameteri(self.id.target.gl_target(), gl::TEXTURE_WRAP_T, wrap as _);
            })
        })
    }

    pub fn wrap_r(&self, wrap: TextureWrap) -> Result<()> {
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::TexParameteri(self.id.target.gl_target(), gl::TEXTURE_WRAP_R, wrap as _);
            })
        })
    }

    pub fn filter_min(&self, param: SampleMode) -> Result<()> {
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::TexParameteri(
                    self.id.target.gl_target(),
                    gl::TEXTURE_MIN_FILTER,
                    param as _,
                );
            })
        })
    }

    pub fn filter_min_mipmap(&self, mipmap: SampleMode, texture: SampleMode) -> Result<()> {
        use SampleMode::*;
        let param = match (mipmap, texture) {
            (Linear, Linear) => gl::LINEAR_MIPMAP_LINEAR,
            (Nearest, Nearest) => gl::NEAREST_MIPMAP_NEAREST,
            (Nearest, Linear) => gl::NEAREST_MIPMAP_LINEAR,
            (Linear, Nearest) => gl::LINEAR_MIPMAP_NEAREST,
        };
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::TexParameteri(
                    self.id.target.gl_target(),
                    gl::TEXTURE_MIN_FILTER,
                    param as _,
                )
            })
        })
    }

    pub fn filter_mag(&self, mode: SampleMode) -> Result<()> {
        gl_error_guard(|| {
            self.with_binding(|| unsafe {
                gl::TexParameteri(
                    self.id.target.gl_target(),
                    gl::TEXTURE_MAG_FILTER,
                    mode as _,
                )
            })
        })
    }
}

#[cfg(feature = "img")]
impl Texture<[f32; 4]> {
    pub fn load_rgba32f<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_repr = path.as_ref().display().to_string();
        tracing::info!("Loading {}", path_repr);
        let img = image::open(path).context("Cannot load image from {}")?;
        Self::from_image(img.to_rgba32f())
    }
}

#[cfg(feature = "img")]
impl Texture<[f32; 3]> {
    pub fn load_rgb32f<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_repr = path.as_ref().display().to_string();
        tracing::info!("Loading {}", path_repr);
        let img = image::open(path).with_context(|| format!("Cannot load image from {}", path_repr))?;
        Self::from_image(img.to_rgb32f())
    }
}

#[cfg(feature = "img")]
impl Texture<[f32; 2]> {
    pub fn load_rg32f<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_repr = path.as_ref().display().to_string();
        tracing::info!("Loading {}", path_repr);
        let mut img = image::open(path)
            .context("Cannot load image from {}")?
            .into_rgb32f();
        image::imageops::flip_vertical_in_place(&mut img);
        let data = img
            .pixels()
            .flat_map(|px| {
                let [r, g, _] = px.0;
                [r, g]
            })
            .collect::<Vec<_>>();
        Self::from_2d_pixels(img.width().try_into()?, &data).context("Cannot upload texture")
    }
}

impl<F: TextureFormat> Texture<F> {}
