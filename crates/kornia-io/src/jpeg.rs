use std::sync::{Arc, Mutex};
use turbojpeg;

use kornia_image::{Image, ImageError, ImageSize};

/// Error types for the JPEG module.
#[derive(thiserror::Error, Debug)]
pub enum JpegError {
    /// Error when the JPEG compressor cannot be created.
    #[error("Something went wrong with the JPEG compressor")]
    TurboJpegError(#[from] turbojpeg::Error),

    /// Error when the image data is not contiguous.
    #[error("Image data is not contiguous")]
    ImageDataNotContiguous,

    /// Error to create the image.
    #[error("Failed to create image")]
    ImageCreationError(#[from] ImageError),
}

/// A JPEG decoder using the turbojpeg library.
pub struct ImageDecoder {
    /// The turbojpeg decompressor.
    pub decompressor: Arc<Mutex<turbojpeg::Decompressor>>,
}

/// A JPEG encoder using the turbojpeg library.
pub struct ImageEncoder {
    /// The turbojpeg compressor.
    pub compressor: Arc<Mutex<turbojpeg::Compressor>>,
}

impl Default for ImageDecoder {
    fn default() -> Self {
        match Self::new() {
            Ok(decoder) => decoder,
            Err(e) => panic!("Failed to create ImageDecoder: {}", e),
        }
    }
}

impl Default for ImageEncoder {
    fn default() -> Self {
        match Self::new() {
            Ok(encoder) => encoder,
            Err(e) => panic!("Failed to create ImageEncoder: {}", e),
        }
    }
}

/// Implementation of the ImageEncoder struct.
impl ImageEncoder {
    /// Creates a new `ImageEncoder`.
    ///
    /// # Returns
    ///
    /// A new `ImageEncoder` instance.
    ///
    /// # Panics
    ///
    /// Panics if the compressor cannot be created.
    pub fn new() -> Result<Self, JpegError> {
        let compressor = turbojpeg::Compressor::new()?;
        Ok(Self {
            compressor: Arc::new(Mutex::new(compressor)),
        })
    }

    /// Encodes the given data into a JPEG image.
    ///
    /// # Arguments
    ///
    /// * `image` - The image to encode.
    ///
    /// # Returns
    ///
    /// The encoded data as `Vec<u8>`.
    pub fn encode(&mut self, image: &Image<u8, 3>) -> Result<Vec<u8>, JpegError> {
        // get the image data
        let image_data = image.as_slice();

        // create a turbojpeg image
        let buf = turbojpeg::Image {
            pixels: image_data,
            width: image.width(),
            pitch: 3 * image.width(),
            height: image.height(),
            format: turbojpeg::PixelFormat::RGB,
        };

        // encode the image
        Ok(self.compressor.lock().unwrap().compress_to_vec(buf)?)
    }

    /// Sets the quality of the encoder.
    ///
    /// # Arguments
    ///
    /// * `quality` - The quality to set.
    pub fn set_quality(&mut self, quality: i32) -> Result<(), JpegError> {
        Ok(self.compressor.lock().unwrap().set_quality(quality)?)
    }
}

/// Implementation of the ImageDecoder struct.
impl ImageDecoder {
    /// Creates a new `ImageDecoder`.
    ///
    /// # Returns
    ///
    /// A new `ImageDecoder` instance.
    pub fn new() -> Result<Self, JpegError> {
        let decompressor = turbojpeg::Decompressor::new()?;
        Ok(ImageDecoder {
            decompressor: Arc::new(Mutex::new(decompressor)),
        })
    }

    /// Reads the header of a JPEG image.
    ///
    /// # Arguments
    ///
    /// * `jpeg_data` - The JPEG data to read the header from.
    ///
    /// # Returns
    ///
    /// The image size.
    ///
    /// # Panics
    ///
    /// Panics if the header cannot be read.
    pub fn read_header(&mut self, jpeg_data: &[u8]) -> Result<ImageSize, JpegError> {
        // read the JPEG header with image size
        let header = self.decompressor.lock().unwrap().read_header(jpeg_data)?;

        Ok(ImageSize {
            width: header.width,
            height: header.height,
        })
    }

    /// Decodes the given JPEG data.
    ///
    /// # Arguments
    ///
    /// * `jpeg_data` - The JPEG data to decode.
    ///
    /// # Returns
    ///
    /// The decoded data as Tensor.
    pub fn decode(&mut self, jpeg_data: &[u8]) -> Result<Image<u8, 3>, JpegError> {
        // get the image size to allocate th data storage
        let image_size = self.read_header(jpeg_data)?;

        // prepare a storage for the raw pixel data
        let mut pixels = vec![0u8; image_size.height * image_size.width * 3];

        // allocate image container
        let buf = turbojpeg::Image {
            pixels: pixels.as_mut_slice(),
            width: image_size.width,
            pitch: 3 * image_size.width, // we use no padding between rows
            height: image_size.height,
            format: turbojpeg::PixelFormat::RGB,
        };

        // decompress the JPEG data
        self.decompressor
            .lock()
            .unwrap()
            .decompress(jpeg_data, buf)?;

        Ok(Image::new(image_size, pixels)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::jpeg::{ImageDecoder, ImageEncoder, JpegError};

    #[test]
    fn image_decoder() -> Result<(), JpegError> {
        let jpeg_data = std::fs::read("../../tests/data/dog.jpeg").unwrap();
        // read the header
        let image_size = ImageDecoder::new()?.read_header(&jpeg_data)?;
        assert_eq!(image_size.width, 258);
        assert_eq!(image_size.height, 195);
        // load the image as file and decode it
        let image = ImageDecoder::new()?.decode(&jpeg_data)?;
        assert_eq!(image.size().width, 258);
        assert_eq!(image.size().height, 195);
        assert_eq!(image.num_channels(), 3);
        Ok(())
    }

    #[test]
    fn image_encoder() -> Result<(), Box<dyn std::error::Error>> {
        let jpeg_data_fs = std::fs::read("../../tests/data/dog.jpeg")?;
        let image = ImageDecoder::new()?.decode(&jpeg_data_fs)?;
        let jpeg_data = ImageEncoder::new()?.encode(&image)?;
        let image_back = ImageDecoder::new()?.decode(&jpeg_data)?;
        assert_eq!(image_back.size().width, 258);
        assert_eq!(image_back.size().height, 195);
        assert_eq!(image_back.num_channels(), 3);
        Ok(())
    }
}
