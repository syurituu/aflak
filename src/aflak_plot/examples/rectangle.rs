extern crate glium;
#[macro_use]
extern crate imgui;
extern crate aflak_imgui_glium_support as support;
extern crate aflak_plot;
extern crate ndarray;

use aflak_plot::{
    imshow::{self, UiImage2d},
    AxisTransform,
};

fn main() {
    let config = support::AppConfig {
        title: "Example rectangle.rs".to_owned(),
        ini_filename: Some(imgui::ImString::new("rectangle.ini")),
        ..Default::default()
    };
    let mut state = imshow::State::default();
    let image_data = {
        const WIDTH: usize = 20;
        const HEIGHT: usize = 10;
        let mut image_data = Vec::with_capacity(WIDTH * HEIGHT);
        for i in 0..HEIGHT {
            for _ in 0..WIDTH {
                image_data.push(i as f32);
            }
        }
        ndarray::Array2::from_shape_vec((HEIGHT, WIDTH), image_data).unwrap()
    };

    support::run(config, |ui, gl_ctx, textures| {
        ui.window(im_str!("Rectangle")).build(|| {
            ui.image2d(
                gl_ctx,
                textures,
                imgui::ImTexture::from(1),
                &image_data,
                "pixel",
                Some(AxisTransform::new("X Axis", |x| x)),
                Some(AxisTransform::new("Y Axis", |y| y)),
                &mut state,
            ).expect("Image2d failed");
        });
        true
    }).unwrap();
}
