use imgui::ImString;

pub struct LayoutEngine {
    outputs: Vec<ImString>,
}

const EDITOR_WINDOW_DEFAULT_POSITION: (f32, f32) = (50.0, 50.0);
const EDITOR_WINDOW_DEFAULT_SIZE: (f32, f32) = (1200.0, 800.0);

const OUTPUT_WINDOW_DEFAULT_SIZE: (f32, f32) = (600.0, 400.0);
const OUTPUT_WINDOW_DEFAULT_MARGIN: f32 = 50.0;

pub struct Layout {
    pub position: (f32, f32),
    pub size: (f32, f32),
}

impl LayoutEngine {
    pub fn new() -> LayoutEngine {
        LayoutEngine { outputs: vec![] }
    }

    pub fn default_editor_window_position(&self) -> (f32, f32) {
        EDITOR_WINDOW_DEFAULT_POSITION
    }

    pub fn default_editor_window_size(&self) -> (f32, f32) {
        EDITOR_WINDOW_DEFAULT_SIZE
    }

    /// Align windows on a 4-column-wide grid
    /// Return None if a window with the given name already exists.
    pub fn default_output_window_position_size(
        &mut self,
        name: &ImString,
        display_size: (f32, f32),
    ) -> Option<Layout> {
        if self.outputs.contains(name) {
            None
        } else {
            let (row, col) = {
                let n = self.outputs.len();
                let row = n / 4;
                let col = n % 4;
                (row as f32, col as f32)
            };
            self.outputs.push(name.clone());
            let pos_x = (EDITOR_WINDOW_DEFAULT_POSITION.0
                + col * (OUTPUT_WINDOW_DEFAULT_SIZE.0 + OUTPUT_WINDOW_DEFAULT_MARGIN))
                % display_size.0;
            let pos_y = (EDITOR_WINDOW_DEFAULT_POSITION.1
                + EDITOR_WINDOW_DEFAULT_SIZE.1
                + OUTPUT_WINDOW_DEFAULT_MARGIN
                + row * (OUTPUT_WINDOW_DEFAULT_SIZE.1 + OUTPUT_WINDOW_DEFAULT_MARGIN))
                % display_size.1;
            Some(Layout {
                position: (pos_x, pos_y),
                size: OUTPUT_WINDOW_DEFAULT_SIZE,
            })
        }
    }
}
