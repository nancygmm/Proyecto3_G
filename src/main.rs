mod framebuffer;
mod ray_intersect;
mod cube;
mod color;
mod camera;
mod light;
mod material;

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
    yellow_light_position: &Vec3, 
    depth: u32,
) -> Color {
    if depth > 3 {
        return adjust_sky_color(yellow_light_position);
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
        return adjust_sky_color(yellow_light_position); 
    }

    let light_dir = (yellow_light_position - intersect.point).normalize();
    let view_dir = (ray_origin - intersect.point).normalize();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalize();

    let shadow_intensity = cast_shadow(&intersect, yellow_light_position, objects);
    let light_intensity = 1.5 * (1.0 - shadow_intensity);

    let diffuse_intensity = intersect.normal.dot(&light_dir).max(0.0).min(1.0);
    let diffuse = intersect.material.diffuse * intersect.material.albedo[0] * diffuse_intensity * light_intensity;

    let specular_intensity = view_dir.dot(&reflect_dir).max(0.0).powf(intersect.material.specular);
    let specular = Color::new(255, 255, 255) * intersect.material.albedo[1] * specular_intensity * light_intensity;

    diffuse + specular
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Object], camera: &Camera, yellow_light_position: &Vec3) {
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

            let pixel_color = cast_ray(&camera.eye, &rotated_direction, objects, yellow_light_position, 0);

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
    )
    .unwrap();

    let green_material = Material::new(
        Color::new(34, 139, 34), 
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );
    
    let brown_material = Material::new(
        Color::new(139, 69, 19), 
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );
    
    let pale_yellow = Material::new(
        Color::new(255, 255, 0), 
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );

    let blue_material = Material::new(
        Color::new(0, 0, 255),
        1.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );


    let mut objects = [
        Object::Cube(Cube { center: Vec3::new(0.0, 10.0, 0.0), size: 1.0, material: pale_yellow }, true), //Sol


        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2 
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2


        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2


        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        
        
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2


        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 0.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 1.0), size: 1.0, material: blue_material }, false), //Lago


        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, -2.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, -2.0), size: 1.0, material: blue_material }, false), //Lago
        
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, -2.0), size: 1.0, material: blue_material }, false), //Lago
        
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, -1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, -1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, -1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 0.0), size: 1.0, material: blue_material }, false), //Lago 
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 0.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 0.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 0.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 1.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(4.0, 2.0, 2.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(5.0, 2.0, 2.0), size: 1.0, material: blue_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(6.0, 2.0, 2.0), size: 1.0, material: blue_material }, false), //Lago


        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 0.0), size: 1.0, material: blue_material }, false), //Lago


        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Lago


        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(8.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Lago
        Object::Cube(Cube { center: Vec3::new(7.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Lago
        


        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2

        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2

        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 0.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(0.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, -2.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(2.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-2.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, 1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(1.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(3.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, 3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-1.0, 1.0, -3.0), size: 1.0, material: green_material }, false), //Tierra2
        Object::Cube(Cube { center: Vec3::new(-3.0, 1.0, -1.0), size: 1.0, material: green_material }, false), //Tierra2


        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Tierra

        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra

        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 0.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(0.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, -2.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(2.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-2.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, 1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(1.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra-
        Object::Cube(Cube { center: Vec3::new(3.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, 3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-1.0, 2.0, -3.0), size: 1.0, material: green_material }, false), //Tierra
        Object::Cube(Cube { center: Vec3::new(-3.0, 2.0, -1.0), size: 1.0, material: green_material }, false), //Tierra
        

        Object::Cube(Cube { center: Vec3::new(0.0, 3.0, 0.0), size: 1.0, material: brown_material }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(0.0, 4.0, 0.0), size: 1.0, material: brown_material }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(0.0, 5.0, 0.0), size: 1.0, material: brown_material }, false), //Tronco


        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, 0.0), size: 1.0, material: brown_material }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, 1.0), size: 1.0, material: green_material }, false), //Hoja

        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, 2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 6.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, 2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, 2.0), size: 1.0, material: green_material }, false), //Hoja

        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 6.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 6.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 6.0, 2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 6.0, 2.0), size: 1.0, material: green_material }, false), //Hoja


        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, 0.0), size: 1.0, material: brown_material }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, 1.0), size: 1.0, material: green_material }, false), //Hoja

        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, 2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 7.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, 2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, 2.0), size: 1.0, material: green_material }, false), //Hoja

        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(2.0, 7.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-2.0, 7.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, -2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 7.0, 2.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 7.0, 2.0), size: 1.0, material: green_material }, false), //Hoja


        Object::Cube(Cube { center: Vec3::new(0.0, 8.0, 0.0), size: 1.0, material: brown_material }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 8.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 8.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 8.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 8.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 8.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 8.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(1.0, 8.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 8.0, 1.0), size: 1.0, material: green_material }, false), //Hoja


        Object::Cube(Cube { center: Vec3::new(0.0, 9.0, 0.0), size: 1.0, material: brown_material }, false), //Tronco
        Object::Cube(Cube { center: Vec3::new(1.0, 9.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(-1.0, 9.0, 0.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 9.0, 1.0), size: 1.0, material: green_material }, false), //Hoja
        Object::Cube(Cube { center: Vec3::new(0.0, 9.0, -1.0), size: 1.0, material: green_material }, false), //Hoja
    ];

    let mut camera = Camera::new(
        Vec3::new(0.0, 5.0, 7.0), 
        Vec3::new(0.0, 5.0, 0.0),  
        Vec3::new(0.0, 3.0, 0.0),  
    );

    let mut angle: f32 = 0.0;
    let radius = 15.0;
    let rotation_speed = 0.05;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        angle += rotation_speed; 
    
        let yellow_light_position = Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
        objects[0] = Object::Cube(Cube { 
            center: yellow_light_position, 
            size: 1.0, 
            material: pale_yellow 
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

        render(&mut framebuffer, &objects, &camera, &yellow_light_position);

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer.width, framebuffer.height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}
