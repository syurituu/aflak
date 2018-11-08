use std::io::Cursor;
use std::path::Path;

pub fn show_frame_and_wave<P: AsRef<Path>>(path: P) -> Cursor<String> {
    let path = path.as_ref().to_string_lossy();
    let ron = format!(
        r#"
(
    dst: (
        transforms: [
            ((6), (
                t: Function("slice_3d_to_2d"),
                input_defaults: [
                    None,
                    None,
                ],
            )),
            ((2), (
                t: Function("open_fits"),
                input_defaults: [
                    None,
                ],
            )),
            ((3), (
                t: Function("fits_to_3d_image"),
                input_defaults: [
                    None,
                ],
            )),
            ((7), (
                t: Function("make_plane3d"),
                input_defaults: [
                    Some(Float3((100, 0, 0))),
                    Some(Float3((0, 0, 1))),
                    Some(Float3((0, 1, 0))),
                    Some(Integer(70)),
                    Some(Integer(70)),
                ],
            )),
            ((4), (
                t: Function("extract_wave"),
                input_defaults: [
                    None,
                    Some(Roi(All)),
                ],
            )),
            ((5), (
                t: Constant([
                    Roi(All),
                ]),
                input_defaults: [
                ],
            )),
            ((1), (
                t: Constant([
                    Path("{}"),
                ]),
                input_defaults: [
                ],
            )),
        ],
        edges: [
            ((
                t_idx: (7),
                output_i: (0),
            ), (
                t_idx: (6),
                input_i: (1),
            )),
            ((
                t_idx: (1),
                output_i: (0),
            ), (
                t_idx: (2),
                input_i: (0),
            )),
            ((
                t_idx: (2),
                output_i: (0),
            ), (
                t_idx: (3),
                input_i: (0),
            )),
            ((
                t_idx: (5),
                output_i: (0),
            ), (
                t_idx: (4),
                input_i: (1),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (4),
                input_i: (0),
            )),
            ((
                t_idx: (3),
                output_i: (0),
            ), (
                t_idx: (6),
                input_i: (0),
            )),
        ],
        outputs: [
            ((2), Some((
                t_idx: (6),
                output_i: (0),
            ))),
            ((1), Some((
                t_idx: (4),
                output_i: (0),
            ))),
        ],
    ),
    node_states: [
        (Transform((1)), (
            selected: false,
            pos: (-785, -596),
            size: (217, 47.5),
        )),
        (Transform((2)), (
            selected: false,
            pos: (-300, -323),
            size: (72, 45.5),
        )),
        (Transform((3)), (
            selected: false,
            pos: (-147, -362),
            size: (121, 45.5),
        )),
        (Transform((4)), (
            selected: false,
            pos: (113, -391),
            size: (93, 45.5),
        )),
        (Transform((5)), (
            selected: false,
            pos: (51, -334),
            size: (44, 28.5),
        )),
        (Transform((6)), (
            selected: false,
            pos: (-48, -241),
            size: (107, 62.5),
        )),
        (Transform((7)), (
            selected: true,
            pos: (-408, -187),
            size: (231, 123.5),
        )),
        (Output((1)), (
            selected: false,
            pos: (314, -426),
            size: (135, 28.5),
        )),
        (Output((2)), (
            selected: false,
            pos: (247, -233),
            size: (135, 28.5),
        )),
    ],
    scrolling: (-818, -667),
)
    "#,
        path
    );
    Cursor::new(ron)
}

#[cfg(test)]
mod test {
    use node_editor::NodeEditor;
    use primitives;

    use super::show_frame_and_wave;
    use constant_editor::MyConstantEditor;

    #[test]
    fn import_frame_and_wave() {
        let transformations_ref = primitives::TRANSFORMATIONS.iter().collect::<Vec<_>>();
        let transformations = transformations_ref.as_slice();

        let buf = show_frame_and_wave("file.fits");
        let editor = NodeEditor::from_export_buf(buf, transformations, MyConstantEditor);
        assert!(editor.is_ok());
    }
}
