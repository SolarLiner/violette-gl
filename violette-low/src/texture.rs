use std::ops::{Deref, DerefMut};
use std::{marker::PhantomData, num::NonZeroU32};

use bytemuck::Pod;
use duplicate::duplicate;
use gl::types::GLenum;
use num_derive::FromPrimitive;

use crate::program::Uniform;
use crate::{
    base::{
        bindable::{BindableExt, Binding, Resource},
        GlType,
    },
    utils::gl_error_guard,
};

pub trait TextureFormat {
    type Subpixel: GlType + Pod;
    const COUNT: usize;
    const FORMAT: GLenum;
    const INTERNAL_FORMAT: GLenum;
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
    const INTERNAL_FORMAT: GLenum = internal_format;
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
    const INTERNAL_FORMAT: GLenum = internal_format;
    const NORMALIZED: bool = false;
}

#[duplicate(
    rust_t      internal_format     format;
    [u8]        [gl::RGB8]           [gl::RGB];
    [i8]        [gl::RGB8I]          [gl::RGB_INTEGER];
    [u16]       [gl::RGB16]          [gl::RGB];
    [i16]       [gl::RGB16I]         [gl::RGB_INTEGER];
    [u32]       [gl::RGB32UI]        [gl::RGB];
    [i32]       [gl::RGB32I]         [gl::RGB_INTEGER];
    [f32]       [gl::RGB32F]         [gl::RGB];
)]
impl TextureFormat for [rust_t; 3] {
    type Subpixel = rust_t;
    const COUNT: usize = 3;
    const FORMAT: GLenum = format;
    const INTERNAL_FORMAT: GLenum = internal_format;
    const NORMALIZED: bool = false;
}

#[duplicate(
    rust_t      internal_format     format;
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
    const INTERNAL_FORMAT: GLenum = internal_format;
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
    const INTERNAL_FORMAT: GLenum = F::INTERNAL_FORMAT;
    const NORMALIZED: bool = true;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureId {
    id: NonZeroU32,
    target: Dimension,
}

impl TextureId {
    pub fn new(id: u32, target: Dimension) -> Option<Self> {
        Some(Self {
            id: NonZeroU32::new(id)?,
            target,
        })
    }
}

impl std::ops::Deref for TextureId {
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
    fn binding_const(&self) -> u32 {
        match self {
            Self::D1 => gl::TEXTURE_BINDING_1D,
            Self::D1Array => gl::TEXTURE_BINDING_1D_ARRAY,
            Self::D2 => gl::TEXTURE_BINDING_2D,
            Self::D2Array => gl::TEXTURE_BINDING_2D_ARRAY,
            Self::D3 => gl::TEXTURE_BINDING_3D,
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
pub struct TextureUnit(pub u32);

impl Uniform for TextureUnit {
    unsafe fn write_uniform(&self, location: gl::types::GLint) {
        tracing::trace!("glUniform1i(<location>, GL_TEXTURE_{})", self.0);
        gl::Uniform1i(location, self.0 as _);
    }
}

#[derive(Debug)]
pub struct Texture<F> {
    __fmt: PhantomData<F>,
    width: u32,
    height: u32,
    depth: u32,
    id: TextureId,
    unit: Option<GLenum>,
}

impl<'a, F: 'a> Resource<'a> for Texture<F> {
    type Id = TextureId;

    type Kind = Dimension;

    type Bound = BoundTexture<'a, F>;

    fn current(kind: Self::Kind) -> Option<Self::Id> {
        let mut id = 0;
        unsafe {
            gl::GetIntegerv(kind.binding_const(), &mut id);
        }
        TextureId::new(id as _, kind)
    }

    fn kind(&self) -> Self::Kind {
        self.id.target
    }

    fn make_binding(&'a mut self) -> anyhow::Result<Self::Bound> {
        unsafe {
            if let Some(unit) = self.unit {
                tracing::trace!("glActiveTexture({:x})", unit);
                gl::ActiveTexture(unit);
            }
            tracing::trace!("glBindTexture({:?}, {})", self.id.target, self.id.get());
            gl::BindTexture(self.id.target as _, self.id.get());
        }
        Ok(BoundTexture { texture: self })
    }
}

impl<F> Texture<F> {
    pub fn new(width: u32, height: u32, depth: u32, target: Dimension) -> Self {
        let mut id = 0;
        unsafe { gl::GenTextures(1, &mut id) }
        Self {
            __fmt: PhantomData,
            width,
            height,
            depth,
            id: TextureId::new(id, target).unwrap(),
            unit: None,
        }
    }

    pub fn wrap_s(&mut self, wrap: TextureWrap) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            gl::TexParameteri(self.id.target as _, gl::TEXTURE_WRAP_S, wrap as _);
        })
    }

    pub fn wrap_t(&mut self, wrap: TextureWrap) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            gl::TexParameteri(self.id.target as _, gl::TEXTURE_WRAP_T, wrap as _);
        })
    }

    pub fn wrap_r(&mut self, wrap: TextureWrap) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            gl::TexParameteri(self.id.target as _, gl::TEXTURE_WRAP_R, wrap as _);
        })
    }

    pub fn filter_min(&mut self, texture: SampleMode, mipmap: SampleMode) -> anyhow::Result<()> {
        use SampleMode::*;
        let param = match (texture, mipmap) {
            (Linear, Linear) => gl::LINEAR_MIPMAP_LINEAR,
            (Nearest, Nearest) => gl::NEAREST_MIPMAP_NEAREST,
            (Nearest, Linear) => gl::NEAREST_MIPMAP_LINEAR,
            (Linear, Nearest) => gl::LINEAR_MIPMAP_NEAREST,
        };
        gl_error_guard(|| unsafe {
            gl::TexParameteri(self.id.target as _, gl::TEXTURE_MIN_FILTER, param as _)
        })
    }

    pub fn filter_mag(&mut self, texture: SampleMode, mipmap: SampleMode) -> anyhow::Result<()> {
        use SampleMode::*;
        let param = match (texture, mipmap) {
            (Linear, Linear) => gl::LINEAR_MIPMAP_LINEAR,
            (Nearest, Nearest) => gl::NEAREST_MIPMAP_NEAREST,
            (Nearest, Linear) => gl::NEAREST_MIPMAP_LINEAR,
            (Linear, Nearest) => gl::LINEAR_MIPMAP_NEAREST,
        };
        gl_error_guard(|| unsafe {
            gl::TexParameteri(self.id.target as _, gl::TEXTURE_MAG_FILTER, param as _)
        })
    }

    pub fn set_texture_unit(&mut self, TextureUnit(off): TextureUnit) {
        self.unit.replace(gl::TEXTURE0 + off);
    }

    pub fn unset_texture_unit(&mut self) {
        self.unit.take();
    }
}

impl<F: TextureFormat> Texture<F> {
    pub fn from_2d_pixels(width: usize, data: &[F::Subpixel]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (data.len() / F::COUNT) % width == 0,
            "Data slice must be a rectangular array of pixels"
        );
        let height = data.len() / F::COUNT / width;
        let mut this = Self::new(width as _, height as _, 1, Dimension::D2);
        this.with_binding(|bound| bound.set_data(data))?;
        Ok(this)
    }

    #[cfg(feature = "img")]
    pub fn from_image<
        P: image::Pixel<Subpixel = F::Subpixel> + AsTextureFormat<TextureFormat = F>,
        C: Deref<Target = [P::Subpixel]> + DerefMut,
    >(
        mut image: image::ImageBuffer<P, C>,
    ) -> anyhow::Result<Self>
    where
        P::Subpixel: GlType + Pod,
    {
        image::imageops::flip_vertical_in_place(&mut image);
        Self::from_2d_pixels(image.width() as _, image.as_raw())
    }
}

pub struct BoundTexture<'a, F> {
    texture: &'a Texture<F>,
}

impl<'a, F> std::ops::Deref for BoundTexture<'a, F> {
    type Target = Texture<F>;

    fn deref(&self) -> &Self::Target {
        self.texture
    }
}

impl<'a, F> Binding<'a> for BoundTexture<'a, F> {
    type Parent = Texture<F>;

    fn unbind(&mut self, previous: Option<TextureId>) {
        unsafe {
            gl::BindTexture(
                self.id.target as _,
                previous.as_ref().map(|id| id.get()).unwrap_or(0),
            );
        }
    }
}

impl<'a, F: TextureFormat> BoundTexture<'a, F> {
    pub fn set_data(&mut self, data: &[F::Subpixel]) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.texture.width * self.texture.height * self.texture.depth * F::COUNT as u32
                == data.len() as _,
            "Data length has to match the extents of the texture"
        );

        let bytes: &[u8] = bytemuck::cast_slice(data);
        gl_error_guard(|| unsafe {
            match self.id.target {
                Dimension::D2 => gl::TexImage2D(
                    self.id.target as _,
                    0,
                    F::INTERNAL_FORMAT as _,
                    self.width as _,
                    self.height as _,
                    0,
                    F::FORMAT,
                    F::Subpixel::GL_TYPE,
                    bytes.as_ptr() as *const _,
                ),
                _ => todo!(),
            }
        })?;
        self.generate_mipmaps()?;
        Ok(())
    }

    pub fn generate_mipmaps(&mut self) -> anyhow::Result<()> {
        gl_error_guard(|| unsafe {
            gl::GenerateMipmap(self.id.target as _);
        })
    }
}
