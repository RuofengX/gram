use gram_core::render::font::FONTS;
use gram_core::render::glyph::{Scale, VecGlyph};
use image::{ImageBuffer, Luma};
use pyo3::{create_exception, prelude::*};
use std::collections::HashSet;
use std::io::Cursor;

create_exception!(gram_tools, AnyhowError, pyo3::exceptions::PyException);

/// A Python module implemented in Rust.
#[pymodule]
fn gram_pytools(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(extract_username, m)?)?;
    m.add_function(wrap_pyfunction!(render_text, m)?)?;
    Ok(())
}

#[pyfunction]
pub fn extract_username(msg: &str) -> PyResult<(HashSet<String>, HashSet<i64>)> {
    gram_core::extract::username::extract_usernames_json(msg)
        .map_err(|e| AnyhowError::new_err(e.to_string()))
}

#[pyfunction]
pub fn render_text(text: String, scale: f32) -> PyResult<Vec<u8>> {
    let vg = VecGlyph::new(&text, Scale::uniform(scale), FONTS.clone());
    let mut img: ImageBuffer<Luma<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(vg.width() as u32, vg.height() as u32, Luma([u8::MAX]));

    vg.draw(|x, y, d| {
        if let Some(pixel) = img.get_pixel_mut_checked(y, x) {
            pixel.0[0] = (d * 255.) as u8;
        }
    });
    let mut ret = Vec::new();
    img.write_to(&mut Cursor::new(&mut ret), image::ImageFormat::Png)
        .unwrap();
    Ok(ret)
}
