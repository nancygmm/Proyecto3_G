// material.rs
use crate::color::Color;
use crate::texture::Texture;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Material {
    pub diffuse: Color,
    pub specular: f32,
    pub albedo: [f32; 4],
    pub refractive_index: f32,
    pub texture: Option<Rc<Texture>>, 
}

impl Material {
    pub fn new(
        diffuse: Color,
        specular: f32,
        albedo: [f32; 4],
        refractive_index: f32,
        texture: Option<Rc<Texture>>, 
    ) -> Self {
        Material {
            diffuse,
            specular,
            albedo,
            refractive_index,
            texture,
        }
    }

    pub fn black() -> Self {
        Material {
            diffuse: Color::black(),
            specular: 0.0,
            albedo: [0.0; 4],
            refractive_index: 0.0,
            texture: None,
        }
    }
}
