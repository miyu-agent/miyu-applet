//! 托盘和菜单用的小图：编进程序的 PNG 解成 RGBA，状态行前面的小圆点现画。托盘库只收 RGBA。

use std::io::Cursor;

/// 一张 RGBA 的图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// 托盘图标（`icons/source/build.sh` 出的）：三个平台都用 logo 留白少的那一份彩色图（象牙色圆角底板）。
/// macOS 上托盘库缩到菜单栏的 22 点高，64px 在两倍屏上也够清楚。
pub const TRAY: &[u8] = include_bytes!("../icons/tray.png");

/// 菜单里的图标（Lucide，`icons/source/lucide/` 是矢量原稿，`icons/source/build.sh` 出图）：灰色线条，macOS 上当模板图用。
pub const STOP: &[u8] = include_bytes!("../icons/menu/square.png");
pub const START: &[u8] = include_bytes!("../icons/menu/play.png");
pub const RESTART: &[u8] = include_bytes!("../icons/menu/rotate-cw.png");
pub const QUIT: &[u8] = include_bytes!("../icons/menu/power.png");

/// 菜单图标、小圆点画多大（像素）：菜单里显示 16 点，高分屏上用两倍的。
pub const MENU_SIZE: u32 = 32;

/// 解一张 PNG，换成 8 位 RGBA。
///
/// # Errors
/// 不是 PNG、坏了、解出来不是灰度或彩色带透明的。
pub fn decode(bytes: &[u8]) -> Result<Rgba, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let size = reader.output_buffer_size().unwrap_or_default();
    let mut buffer = vec![0; size];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| error.to_string())?;
    buffer.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::GrayscaleAlpha => buffer
            .chunks_exact(2)
            .flat_map(|pair| [pair[0], pair[0], pair[0], pair[1]])
            .collect(),
        other => return Err(format!("unexpected colour type {other:?}")),
    };
    Ok(Rgba {
        pixels,
        width: info.width,
        height: info.height,
    })
}

/// 状态行前面的小圆点：边长 `size` 的透明方块，正中一个直径约四成的实心圆，边缘照覆盖的比例渐隐。
pub fn dot(rgb: [u8; 3], size: u32) -> Rgba {
    let radius = f64::from(size) * 0.2;
    let center = f64::from(size) / 2.0;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = f64::from(x) + 0.5 - center;
            let dy = f64::from(y) + 0.5 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let cover = (radius + 0.5 - distance).clamp(0.0, 1.0);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "cover 夹在 0 到 1 之间，乘 255 落在 u8 里"
            )]
            let alpha = (cover * 255.0).round() as u8;
            pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
        }
    }
    Rgba {
        pixels,
        width: size,
        height: size,
    }
}

#[cfg(test)]
mod tests;
