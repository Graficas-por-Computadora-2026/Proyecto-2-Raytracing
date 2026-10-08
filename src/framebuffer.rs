use raylib::prelude::*;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    texture: Texture2D,
}

impl Framebuffer {
    pub fn new(
        window: &mut RaylibHandle,
        thread: &RaylibThread,
        width: usize,
        height: usize,
    ) -> Self {
        let image = Image::gen_image_color(width as i32, height as i32, Color::new(8, 16, 28, 255));
        let texture = window
            .load_texture_from_image(thread, &image)
            .expect("No se pudo crear el framebuffer");
        texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
        Self {
            width,
            height,
            pixels: vec![0; width * height * 4],
            texture,
        }
    }
    pub fn upload(&mut self) {
        self.texture
            .update_texture(&self.pixels)
            .expect("No se pudo actualizar el framebuffer");
    }
    pub fn draw(&self, d: &mut RaylibDrawHandle, window_width: i32, window_height: i32) {
        d.draw_texture_pro(
            &self.texture,
            Rectangle::new(0., 0., self.width as f32, self.height as f32),
            Rectangle::new(0., 0., window_width as f32, window_height as f32),
            Vector2::zero(),
            0.,
            Color::WHITE,
        );
    }
}
