// Drawing text with outline
// TODO: should be moved into the engine

use super::*;

pub const SHADER_SOURCE: &'static str = "
varying vec2 v_uv;
#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 i_pos;
attribute vec2 i_size;
attribute vec2 i_uv_pos;
attribute vec2 i_uv_size;
uniform mat3 u_projection_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_model_matrix;
void main() {
    v_uv = i_uv_pos + a_pos * i_uv_size;
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(i_pos + a_pos * i_size, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif
#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
uniform vec4 u_color;
uniform vec4 u_outline_color;
float aa(float edge, float x) {
    float w = length(vec2(dFdx(x), dFdy(x)));
    return smoothstep(edge - w, edge + w, x);
}
void main() {
    float dist = (texture2D(u_texture, v_uv).x - 0.5) * 2.0;
    float w = length(vec2(dFdx(dist), dFdy(dist)));
    float inside = aa(0.0, dist);
    float inside_border = aa(-0.15, dist);
    gl_FragColor = u_color * inside + (1.0 - inside) * (u_outline_color * inside_border + vec4(u_outline_color.xyz, 0.0) * (1.0 - inside_border));
}
#endif
";

pub struct Text<'a, F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> {
    geng: Geng,
    program: &'a ugli::Program,
    inner: draw2d::Text<F, T>,
    outline_color: Rgba<f32>,
}

impl<'a, F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> Text<'a, F, T> {
    pub fn unit(
        geng: &Geng,
        program: &'a ugli::Program,
        font: F,
        text: T,
        color: Rgba<f32>,
        outline_color: Rgba<f32>,
    ) -> Self {
        Self {
            geng: geng.clone(),
            program,
            inner: draw2d::Text::unit(font, text, color),
            outline_color,
        }
    }
}

impl<F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> Transform2d<f32> for Text<'_, F, T> {
    fn bounding_quad(&self) -> Quad<f32> {
        self.inner.bounding_quad()
    }
    fn apply_transform(&mut self, transform: mat3<f32>) {
        self.inner.apply_transform(transform)
    }
}

impl<F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> geng::Draw2d for Text<'_, F, T> {
    fn draw2d_transformed(
        &self,
        _geng: &Geng,
        framebuffer: &mut ugli::Framebuffer,
        camera: &dyn geng::AbstractCamera2d,
        transform: mat3<f32>,
    ) {
        let transform = transform * self.inner.transform * self.inner.into_unit_transform;
        let size = 1000.0;
        let transform = transform * mat3::scale_uniform(size);
        self.inner.font.borrow().draw_with(
            self.inner.text.as_ref(),
            vec2(geng::TextAlign::LEFT, geng::TextAlign::LEFT),
            |glyphs, texture| {
                let framebuffer_size = framebuffer.size();
                ugli::draw(
                    framebuffer,
                    self.program,
                    ugli::DrawMode::TriangleFan,
                    // TODO: don't create VBs each time
                    ugli::instanced(
                        &ugli::VertexBuffer::new_dynamic(
                            self.geng.ugli(),
                            Aabb2::point(vec2::ZERO)
                                .extend_positive(vec2(1.0, 1.0))
                                .corners()
                                .into_iter()
                                .map(|v| draw2d::Vertex { a_pos: v })
                                .collect(),
                        ),
                        &ugli::VertexBuffer::new_dynamic(self.geng.ugli(), glyphs.to_vec()),
                    ),
                    (
                        ugli::uniforms! {
                            u_texture: texture,
                            u_model_matrix: transform,
                            u_color: self.inner.color,
                            u_outline_color: self.outline_color,
                        },
                        geng::camera2d_uniforms(camera, framebuffer_size.map(|x| x as f32)),
                    ),
                    ugli::DrawParameters {
                        depth_func: None,
                        blend_mode: Some(ugli::BlendMode::straight_alpha()),
                        ..default()
                    },
                );
            },
        );
    }
}
