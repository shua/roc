use crate::editor::colors::CODE_COLOR;
use crate::{
    graphics::{
        primitives::text::{queue_code_text_draw, Text},
        style::CODE_FONT_SIZE,
    },
    lang::{ast::Expr2, expr::Env},
};
use cgmath::Vector2;
use wgpu_glyph::GlyphBrush;
use winit::dpi::PhysicalSize;

pub fn render_expr2<'a>(
    env: &mut Env<'a>,
    expr2: &Expr2,
    size: &PhysicalSize<u32>,
    position: Vector2<f32>,
    glyph_brush: &mut GlyphBrush<()>,
) {
    let area_bounds = (size.width as f32, size.height as f32).into();

    match expr2 {
        Expr2::SmallInt { text, .. } => {
            let code_text = Text {
                position,
                area_bounds,
                color: CODE_COLOR.into(),
                text: env.pool.get_str(text),
                size: CODE_FONT_SIZE,
                ..Default::default()
            };

            queue_code_text_draw(&code_text, glyph_brush);
        }
        Expr2::I128 { text, .. } => {
            let code_text = Text {
                position,
                area_bounds,
                color: CODE_COLOR.into(),
                text: env.pool.get_str(text),
                size: CODE_FONT_SIZE,
                ..Default::default()
            };

            queue_code_text_draw(&code_text, glyph_brush);
        }
        Expr2::U128 { text, .. } => {
            let code_text = Text {
                position,
                area_bounds,
                color: CODE_COLOR.into(),
                text: env.pool.get_str(text),
                size: CODE_FONT_SIZE,
                ..Default::default()
            };

            queue_code_text_draw(&code_text, glyph_brush);
        }
        Expr2::Float { text, .. } => {
            let code_text = Text {
                position,
                area_bounds,
                color: CODE_COLOR.into(),
                text: env.pool.get_str(text),
                size: CODE_FONT_SIZE,
                ..Default::default()
            };

            queue_code_text_draw(&code_text, glyph_brush);
        }
        Expr2::Str(text) => {
            let code_text = Text {
                position,
                area_bounds,
                color: CODE_COLOR.into(),
                text: env.pool.get_str(text),
                size: CODE_FONT_SIZE,
                ..Default::default()
            };

            queue_code_text_draw(&code_text, glyph_brush);
        }
        Expr2::GlobalTag { name, .. } => {
            let code_text = Text {
                position,
                area_bounds,
                color: CODE_COLOR.into(),
                text: env.pool.get_str(name),
                size: CODE_FONT_SIZE,
                ..Default::default()
            };

            queue_code_text_draw(&code_text, glyph_brush);
        }
        Expr2::Call { expr: expr_id, .. } => {
            let expr = env.pool.get(*expr_id);

            render_expr2(env, expr, size, position, glyph_brush);
        }
        Expr2::Var(symbol) => {
            let text = format!("{:?}", symbol);

            let code_text = Text {
                position,
                area_bounds,
                color: CODE_COLOR.into(),
                text: text.as_str(),
                size: CODE_FONT_SIZE,
                ..Default::default()
            };

            queue_code_text_draw(&code_text, glyph_brush);
        }
        rest => todo!("implement {:?} render", rest),
    };
}
