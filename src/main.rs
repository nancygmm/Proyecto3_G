mod framebuffer;
mod ray_intersect;
mod cube;
mod color;
mod camera;
mod light;
mod material;
mod texture;

use minifb::{Window, WindowOptions, Key};
use nalgebra_glm::{Vec3, normalize};
use std::time::Duration;
use std::f32::consts::PI;
use crate::color::Color;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::cube::Cube;
use crate::framebuffer::Framebuffer;
use crate::camera::Camera;
use crate::material::Material;
use crate::texture::Texture;
use std::rc::Rc;

const ORIGIN_BIAS: f32 = 1e-4;
const DAY_SKY_COLOR: Color = Color::new(68, 142, 228);
const NIGHT_SKY_COLOR: Color = Color::new(10, 10, 30);

fn offset_origin(intersect: &Intersect, direction: &Vec3) -> Vec3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(&intersect.normal) < 0.0 {
        intersect.point - offset
    } else {
        intersect.point + offset
    }
}

fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * incident.dot(normal) * normal
}

fn cast_shadow(
    intersect: &Intersect,
    light_position: &Vec3,
    objects: &[Object],
) -> f32 {
    let light_dir = (light_position - intersect.point).normalize();
    let light_distance = (light_position - intersect.point).magnitude();
    let shadow_ray_origin = offset_origin(intersect, &light_dir);
    let mut shadow_intensity = 0.0;

    for object in objects {
        let shadow_intersect = match object {
            Object::Cube(cube, _) => cube.ray_intersect(&shadow_ray_origin, &light_dir),
        };
        if shadow_intersect.is_intersecting && shadow_intersect.distance < light_distance {
            let distance_ratio = shadow_intersect.distance / light_distance;
            shadow_intensity = 1.0 - distance_ratio.powf(2.0).min(1.0);
            break;
        }
    }

    shadow_intensity
}

enum Object {
    Cube(Cube, bool),
}

fn adjust_sky_color(sun_position: &Vec3) -> Color {
    if sun_position.y > 0.0 {
        DAY_SKY_COLOR
    } else {
        NIGHT_SKY_COLOR
    }
}

pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Object],
    sun_position: &Vec3,
    sun_intensity: f32,
    depth: u32,
) -> Color {
    if depth > 3 {
        return adjust_sky_color(sun_position);
    }

    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let i = match object {
            Object::Cube(cube, _) => cube.ray_intersect(ray_origin, ray_direction),
        };
        if i.is_intersecting && i.distance < zbuffer {
            zbuffer = i.distance;
            intersect = i;
        }
    }

    if !intersect.is_intersecting {
        return adjust_sky_color(sun_position);
    }

    let light_dir = (sun_position - intersect.point).normalize();
    let view_dir = (ray_origin - intersect.point).normalize();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalize();

    let shadow_intensity = cast_shadow(&intersect, sun_position, objects);


    let sun_height = sun_position.y.max(0.0);
    let light_intensity = if sun_height > 0.0 {
        sun_intensity * (sun_height / 15.0) + 1.0 
    } else {
        0.0
    };

    let diffuse_intensity = intersect.normal.dot(&light_dir).abs().max(0.5);
    let specular_intensity = view_dir.dot(&reflect_dir).max(0.0).powf(intersect.material.specular);

    let diffuse_color = if let Some(texture) = &intersect.material.texture {
        let (u, v) = intersect.uv.unwrap();
        let [r, g, b] = texture.get_color(u, v);
        Color::new(r, g, b)
    } else {
        intersect.material.diffuse
    };

    let ambient_light = if sun_position.y < 0.0 { 0.3 } else { 0.2 };

    let diffuse = diffuse_color * intersect.material.albedo[0] * diffuse_intensity * light_intensity * (1.0 - shadow_intensity);
    let specular = Color::new(255, 255, 255) * intersect.material.albedo[1] * specular_intensity * light_intensity * (1.0 - shadow_intensity);
    let ambient = diffuse_color * ambient_light;

    diffuse + specular + ambient
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Object], camera: &Camera, sun_position: &Vec3, sun_intensity: f32) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));
            let rotated_direction = camera.base_change(&ray_direction);

            let pixel_color = cast_ray(&camera.eye, &rotated_direction, objects, sun_position, sun_intensity, 0);

            framebuffer.set_current_color(pixel_color.to_hex());
            framebuffer.point(x, y);
        }
    }
}

fn main() {
    let window_width = 800;
    let window_height = 600;
    let framebuffer_width = 800;
    let framebuffer_height = 600;
    let frame_delay = Duration::from_millis(16);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);

    let mut window = Window::new(
        "Refractor",
        window_width,
        window_height,
        WindowOptions::default(),
    ).unwrap();

    let grass_texture = Rc::new(Texture::new("src/Grass.png"));
    let dirt_texture = Rc::new(Texture::new("src/Dirt.png"));
    let leaves_texture = Rc::new(Texture::new("src/Leaves.png"));
    let trunk_texture = Rc::new(Texture::new("src/Trunk.png"));
    let sun_texture = Rc::new(Texture::new("src/SunMoon.png"));
    let water_texture = Rc::new(Texture::new("src/Water.png"));
    let hive_texture = Rc::new(Texture::new("src/Hive.png"));
    let stone_texture = Rc::new(Texture::new("src/Stone.png"));

    let grass_material = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(grass_texture.clone()),
    );

    let dirt_material = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(dirt_texture.clone()),
    );

    let leaves_material = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(leaves_texture.clone()),
    );

    let trunk_material = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(trunk_texture.clone()),
    );

    let pale_yellow = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(sun_texture.clone())
    );

    let water_material = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(water_texture.clone())
    );

    let hive_material = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(hive_texture.clone())
    );

    let stone_material = Material::new(
        Color::black(),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
        Some(stone_texture.clone())
    );

    let mut objects = [
        Object::Cube(Cube { center: Vec3::new(0.0, 10.0, 0.0), size: 1.0, material: pale_yellow.clone() }, true), //Sol


        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 0.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, -2.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, -2.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, -2.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, -1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, -1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, -1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 0.0), size: 1.0, material: water_material.clone() }, false), //Lago 
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 0.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 0.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 0.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 1.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 2.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 2.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 2.0), size: 1.0, material: water_material.clone() }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 0.0), size: 1.0, material: water_material.clone() }, false), //Lago


        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, -3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, -3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, -3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -2.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 2.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 0.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, -2.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, -3.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, -1.0), size: 1.0, material: stone_material.clone() }, false), //Tierra2


        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 0.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 0.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 0.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, -1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, -1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, -1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 0.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 0.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, -2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, -2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, -2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, -1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, -1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, -2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, -2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 0.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 0.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, -3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, -3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, -2.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, -3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, -3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, -3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, -3.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, -1.0), size: 1.0, material: dirt_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, -3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, -2.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, -1.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 0.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 1.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 2.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 2.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -2.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 3.0), size: 1.0, material: grass_material.clone() }, false), //Tierra
        

        Object::Cube(Cube { center: Vec3::new(0.0, 3.0, 0.0), size: 1.0, material: trunk_material.clone() }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(0.0, 4.0, 0.0), size: 1.0, material: trunk_material.clone() }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(0.0, 5.0, 0.0), size: 1.0, material: trunk_material.clone() }, false), //Tronco


        Object::Cube(Cube { center: Vec3::new(1.0, 5.0, 0.0), size: 1.0, material: hive_material.clone() }, false), //Hive


        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, 0.0), size: 1.0, material: trunk_material.clone() }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja


        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, 0.0), size: 1.0, material: trunk_material.clone() }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, -2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, 2.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja


        Object::Cube(Cube { center: Vec3::new(0.0, 8.0, 0.0), size: 1.0, material: trunk_material.clone() }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 8.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 8.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 8.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 8.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 8.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 8.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 8.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 8.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja


        Object::Cube(Cube { center: Vec3::new(0.0, 9.0, 0.0), size: 1.0, material: trunk_material.clone() }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 9.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 9.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 9.0, 1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 9.0, -1.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 10.0, 0.0), size: 1.0, material: leaves_material.clone() }, false), //Hoja
    ];

    let mut camera = Camera::new(
        Vec3::new(0.0, 5.0, 7.0),
        Vec3::new(0.0, 5.0, 0.0),
        Vec3::new(0.0, 3.0, 0.0),
    );

    let mut angle: f32 = 0.0;
    let radius = 15.0;
    let rotation_speed = 0.05;
    let sun_intensity = 2.0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        angle += rotation_speed;

        let sun_position = Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
        objects[0] = Object::Cube(Cube {
            center: sun_position,
            size: 1.0,
            material: pale_yellow.clone(),
        }, true);

        if window.is_key_down(Key::W) {
            camera.move_camera("forward");
        }
        if window.is_key_down(Key::S) {
            camera.move_camera("backward");
        }
        if window.is_key_down(Key::A) {
            camera.move_camera("left");
        }
        if window.is_key_down(Key::D) {
            camera.move_camera("right");
        }
        if window.is_key_down(Key::Left) {
            camera.orbit(rotation_speed, 0.0);
        }
        if window.is_key_down(Key::Right) {
            camera.orbit(-rotation_speed, 0.0);
        }
        if window.is_key_down(Key::Up) {
            camera.orbit(0.0, -rotation_speed);
        }
        if window.is_key_down(Key::Down) {
            camera.orbit(0.0, rotation_speed);
        }

        render(&mut framebuffer, &objects, &camera, &sun_position, sun_intensity);

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer.width, framebuffer.height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}