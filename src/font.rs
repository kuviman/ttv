use super::*;

const SHADER_SOURCE: &'static str = "
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

pub struct Text<F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> {
    geng: Geng,
    program: ugli::Program,
    inner: draw_2d::Text<F, T>,
    outline_color: Rgba<f32>,
}

impl<F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> Text<F, T> {
    pub fn unit(geng: &Geng, font: F, text: T, color: Rgba<f32>, outline_color: Rgba<f32>) -> Self {
        Self {
            geng: geng.clone(),
            program: geng.shader_lib().compile(SHADER_SOURCE).unwrap(), // TODO
            inner: draw_2d::Text::unit(font, text, color),
            outline_color,
        }
    }
}

impl<F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> Transform2d<f32> for Text<F, T> {
    fn bounding_quad(&self) -> Quad<f32> {
        self.inner.bounding_quad()
    }
    fn apply_transform(&mut self, transform: Mat3<f32>) {
        self.inner.apply_transform(transform)
    }
}

impl<F: std::borrow::Borrow<geng::Font>, T: AsRef<str>> geng::Draw2d for Text<F, T> {
    fn draw_2d_transformed(
        &self,
        geng: &Geng,
        framebuffer: &mut ugli::Framebuffer,
        camera: &dyn geng::AbstractCamera2d,
        transform: Mat3<f32>,
    ) {
        // self.font.borrow().draw_impl(
        //     framebuffer,
        //     camera,
        //     transform * self.transform * self.into_unit_transform,
        //     self.text.as_ref(),
        //     vec2(0.0, 0.0),
        //     SIZE_HACK,
        //     self.color,
        // );

        let transform = transform * self.inner.transform * self.inner.into_unit_transform;
        let size = 1000.0;
        let transform = transform * Mat3::scale_uniform(size);
        self.inner
            .font
            .borrow()
            .draw_with(self.inner.text.as_ref(), |glyphs, texture| {
                let framebuffer_size = framebuffer.size();
                ugli::draw(
                    framebuffer,
                    &self.program,
                    ugli::DrawMode::TriangleFan,
                    // TODO: don't create VBs each time
                    ugli::instanced(
                        &ugli::VertexBuffer::new_dynamic(
                            self.geng.ugli(),
                            AABB::point(Vec2::ZERO)
                                .extend_positive(vec2(1.0, 1.0))
                                .corners()
                                .into_iter()
                                .map(|v| draw_2d::Vertex { a_pos: v })
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
                        blend_mode: Some(ugli::BlendMode::default()),
                        ..default()
                    },
                );
            });
    }
}
