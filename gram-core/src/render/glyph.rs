pub use rusttype::Scale;
use rusttype::{Font, ScaledGlyph, point};

pub struct VecGlyph<'fonts>(Vec<ScaledGlyph<'fonts>>);
impl<'fonts> VecGlyph<'fonts> {
    pub fn new(
        text: &str,
        scale: Scale,
        fonts: impl IntoIterator<Item = Font<'fonts>> + Clone,
    ) -> VecGlyph<'fonts> {
        let chars = text.chars().collect::<Vec<_>>();

        let mut fonts = fonts.into_iter().peekable();
        let default_f = fonts.peek().expect("未提供字体").clone();

        let mut found_map = vec![false; chars.len()];
        let mut ret = vec![default_f.glyph('?'); chars.len()];

        fonts.into_iter().for_each(|f| {
            f.glyphs_for(chars.iter().cloned())
                .enumerate()
                .for_each(|(i, g)| {
                    if g.id().0 == 0 {
                        return;
                    }
                    if !found_map[i] {
                        ret[i] = g;
                        found_map[i] = true;
                    } else {
                        // println!("未找到字符:{:?}", i);
                    }
                })
        });

        let ret = ret.into_iter().map(|g| g.scaled(scale)).collect();

        Self(ret)
    }

    pub fn height(&self) -> f32 {
        // self.0
        //     .iter()
        //     .map(|x| {
        //         x.exact_bounding_box()
        //             .map(|bb| bb.max.y - bb.min.y)
        //             .unwrap_or(0.0)
        //     })
        //     .reduce(f32::max)
        //     .unwrap_or(0.)
        self.0
            .iter()
            .map(|g| {
                let vm = g.font().v_metrics(g.scale());
                vm.ascent - vm.descent + vm.line_gap
            })
            .reduce(f32::max)
            .unwrap_or(0.)
    }
    pub fn width(&self) -> f32 {
        self.0
            .iter()
            .map(|x| x.h_metrics().advance_width)
            .sum::<f32>()
    }

    pub fn draw(self, mut drawable: impl FnMut(u32, u32, f32)) {
        let mut h_offset = 0.;
        let height = self.height();
        for g in self.0 {
            let offset_delta = g.h_metrics().advance_width;
            let pg = g.positioned(point(0., 0.));
            let g_height = pg
                .pixel_bounding_box()
                .map(|bb| bb.max.y - bb.min.y)
                .unwrap_or(0) as u32;
            let g_down = (height - g_height as f32) / 2.;

            // height as u32 - g_height + ascent as u32;

            pg.draw(|x, y, v| {
                drawable(y + g_down as u32, x + h_offset as u32, 1.0 - v);
            });
            h_offset += offset_delta;
        }
    }
}
