// texture.rs
use image::{DynamicImage, GenericImageView};
use std::path::Path;

#[derive(Debug)] 
pub struct Texture {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn new(filename: &str) -> Self {
        let img = image::open(&Path::new(filename)).expect("Failed to load texture");
        let (width, height) = img.dimensions();
        Texture {
            image: img,
            width,
            height,
        }
    }

    pub fn get_color(&self, u: f32, v: f32) -> [u8; 3] {
        let u = u.fract();
        let v = v.fract();

        let x = (u * self.width as f32) as u32 % self.width;
        let y = ((1.0 - v) * self.height as f32) as u32 % self.height;

        let pixel = self.image.get_pixel(x, y);
        [pixel[0], pixel[1], pixel[2]]
    }
}
