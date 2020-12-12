use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

#[derive(Clone, Copy)]
struct LinearRGB {
    r: f32,
    g: f32,
    b: f32,
}

struct ImgBuf {
    width: usize,
    height: usize,
    buf: Vec<u8>,
}

fn srgb_inv_gamma(u: f32) -> f32 {
    if u <= 0.04045 {
        u / 12.92
    } else {
        ((u + 0.055) / 1.055).powf(2.4)
    }
}

impl LinearRGB {
    fn from_srgb(r: u8, g: u8, b: u8) -> LinearRGB {
        fn inv(x: u8) -> f32 {
            srgb_inv_gamma((x as f32) * (1.0 / 255.0))
        }
        LinearRGB {
            r: inv(r),
            g: inv(g),
            b: inv(b),
        }
    }

    fn alpha_blend(self, top: LinearRGB, alpha: f32) -> LinearRGB {
        fn lerp(x0: f32, x1: f32, t: f32) -> f32 {
            x0 + (x1 - x0) * t
        }
        LinearRGB {
            r: lerp(self.r, top.r, alpha),
            g: lerp(self.g, top.g, alpha),
            b: lerp(self.b, top.b, alpha),
        }
    }

    fn kubelka_blend(self, top: LinearRGB, alpha: f32) -> LinearRGB {
        fn kubelka(x0: f32, x1: f32, t: f32) -> f32 {
            let k = x1 * t; // reflectance over black
            let w = k + 1.0 - t; // reflectance over white
            k + x0 * (w - k) * (1.0 - k) / (1.0 - x0 * k)
        }
        LinearRGB {
            r: kubelka(self.r, top.r, alpha),
            g: kubelka(self.g, top.g, alpha),
            b: kubelka(self.b, top.b, alpha),
        }
    }
}

impl ImgBuf {
    fn new(width: usize, height: usize) -> ImgBuf {
        ImgBuf {
            width,
            height,
            buf: vec![255; width * height * 4],
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: LinearRGB) {
        fn gamma(u: f32) -> u8 {
            let z = if u <= 0.0031308 {
                12.92 * u
            } else {
                1.055 * u.powf(1.0 / 2.4) - 0.055
            };
            (z.max(0.0).min(1.0) * 255.0).round() as u8
        }
        // TODO: should bounds-check
        let ix = (y * self.width + x) * 4;
        self.buf[ix + 0] = gamma(color.r);
        self.buf[ix + 1] = gamma(color.g);
        self.buf[ix + 2] = gamma(color.b);
    }

    fn write_img(&self, filename: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let file = File::create(filename)?;
        let w = BufWriter::new(file);
    
        let mut encoder = png::Encoder::new(w, self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
    
        writer.write_image_data(&self.buf)?;
        Ok(())
    }
}

fn make_blend() -> ImgBuf {
    let width = 640;
    let height = 600;
    let stripe = 600;
    let mut buf = ImgBuf::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let g = srgb_inv_gamma((x as f32) / (width as f32));
            //let color = LinearRGB { r: g, g: g, b: g };
            let color = LinearRGB { r: g, g: g, b: g };
            let top = LinearRGB::from_srgb(255, 128, 0);
            let alpha = 1.0 - ((y % stripe) as f32) / (stripe as f32);
            let color = if y % stripe < 2 {
                LinearRGB::from_srgb(0, 0, 0)
            } else if x % 64 > 32 /* y % 600 >= 300 */ {
                color.kubelka_blend(top, alpha)
            } else {
                color.alpha_blend(top, alpha)
            };
            buf.set_pixel(x, y, color);
        }
    }
    buf
}

fn main() {
    let buf = make_blend();
    buf.write_img("out.png").unwrap();
}
