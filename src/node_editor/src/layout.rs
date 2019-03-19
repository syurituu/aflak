use std::error::Error;

use imgui::{
    ImGuiCol, ImGuiKey, ImGuiMouseCursor, ImGuiSelectableFlags, ImMouseButton, ImString, ImVec2,
    StyleVar, Ui, WindowDrawList,
};
use serde::{Deserialize, Serialize};

use cake::{self, InputSlot, Transform, VariantName, DST};

use constant_editor::ConstantEditor;
use event::RenderEvent;
use id_stack::GetId;
use node_state::NodeStates;
use scrolling::Scrolling;
use vec2::Vec2;

pub struct NodeEditorLayout<T: 'static, E: 'static> {
    node_states: NodeStates,
    active_node: Option<cake::NodeId>,
    drag_node: Option<cake::NodeId>,
    creating_link: Option<LinkExtremity>,
    new_link: Option<(cake::Output, InputSlot)>,
    show_left_pane: bool,
    left_pane_size: Option<f32>,
    show_top_pane: bool,
    show_connection_names: bool,
    scrolling: Scrolling,
    show_grid: bool,

    // Used at runtime to aggregate events
    events: Vec<RenderEvent<T, E>>,
}

impl<T, E> Default for NodeEditorLayout<T, E> {
    fn default() -> Self {
        NodeEditorLayout {
            node_states: NodeStates::new(),
            active_node: None,
            drag_node: None,
            creating_link: None,
            new_link: None,
            show_left_pane: true,
            left_pane_size: None,
            show_top_pane: true,
            show_connection_names: true,
            scrolling: Default::default(),
            show_grid: true,

            events: vec![],
        }
    }
}

pub enum LinkExtremity {
    Output(cake::Output),
    Input(InputSlot),
}

const NODE_FRAME_COLOR: [f32; 3] = [0.39, 0.39, 0.39];
const NODE_WINDOW_PADDING: Vec2 = Vec2(5.0, 5.0);
const CURRENT_FONT_WINDOW_SCALE: f32 = 1.0;

impl<T, E> NodeEditorLayout<T, E>
where
    T: 'static
        + Clone
        + cake::EditableVariants
        + cake::NamedAlgorithms<E>
        + cake::VariantName
        + cake::DefaultFor
        + cake::ConvertibleVariants
        + Serialize
        + for<'de> Deserialize<'de>,
    E: 'static + Error,
{
    /// Draw the full node editor on the current window.
    pub fn render<ED>(
        &mut self,
        ui: &Ui,
        dst: &DST<'static, T, E>,
        addable_nodes: &[&'static Transform<T, E>],
        addable_macros: &cake::macros::MacroManager<'static, T, E>,
        constant_editor: &ED,
    ) -> Vec<RenderEvent<T, E>>
    where
        ED: ConstantEditor<T>,
    {
        self.events = vec![];

        for idx in dst.node_ids() {
            // Initialization of node states
            let win_pos: Vec2 = ui.get_cursor_screen_pos().into();
            let scroll = self.scrolling.get_current();
            let clue = if ui.is_window_focused() {
                let offset = win_pos - scroll;
                let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                mouse_pos * 0.7 - offset
            } else {
                scroll + Vec2(30.0, 30.0)
            };
            self.node_states.init_node(&idx, clue);
        }
        if self.show_left_pane {
            self.render_left_pane(ui, dst);
        }
        self.render_graph_node(ui, dst, addable_nodes, addable_macros, constant_editor);

        if ui.is_window_focused() && !ui.want_capture_keyboard() {
            let delete_index = ui.imgui().get_key_index(ImGuiKey::Delete);
            let backspace_index = ui.imgui().get_key_index(ImGuiKey::Backspace);
            if ui.imgui().is_key_pressed(delete_index) || ui.imgui().is_key_pressed(backspace_index)
            {
                self.delete_selected_nodes();
            }
        }
        self.scrolling.tick();

        ::std::mem::replace(&mut self.events, vec![])
    }

    fn render_left_pane(&mut self, ui: &Ui, dst: &DST<'static, T, E>) {
        const LEFT_PANE_DEFAULT_RELATIVE_WIDTH: f32 = 0.2;
        let window_size = Vec2::new(ui.get_window_size());
        let pane_width = *self
            .left_pane_size
            .get_or_insert_with(|| window_size.0 * LEFT_PANE_DEFAULT_RELATIVE_WIDTH);

        ui.child_frame(im_str!("node_list"), (pane_width, 0.0))
            .build(|| {
                ui.spacing();
                ui.separator();
                if ui
                    .collapsing_header(im_str!("Node List##node_list_1"))
                    .default_open(true)
                    .build()
                {
                    ui.separator();
                    self.show_node_list(ui, dst);
                }
                ui.separator();
                if let Some(node_id) = self.active_node {
                    ui.spacing();
                    ui.separator();
                    if ui
                        .collapsing_header(im_str!("Active Node##activeNode"))
                        .default_open(true)
                        .build()
                    {
                        ui.separator();
                        let node = dst.get_node(&node_id).expect("Failed to get active node");
                        match node {
                            cake::Node::Transform(t) => {
                                ui.text_wrapped(&ImString::new(format!(
                                    "{}:\n{}",
                                    node.name(&node_id),
                                    t.description()
                                )));
                            }
                            cake::Node::Output(_) => {
                                ui.text(format!("{}", node.name(&node_id)));
                            }
                        }
                    }
                    ui.separator();
                }
            });

        // Horizontal splitter
        ui.same_line(0.0);
        const SPLITTER_WIDTH: f32 = 6.0;
        const SPLITTER_DESIGN: [(ImGuiCol, (f32, f32, f32, f32)); 3] = [
            (ImGuiCol::Button, (1.0, 1.0, 1.0, 0.2)),
            (ImGuiCol::ButtonHovered, (1.0, 1.0, 1.0, 0.35)),
            (ImGuiCol::ButtonActive, (1.0, 1.0, 1.0, 0.5)),
        ];
        ui.with_color_vars(&SPLITTER_DESIGN, || {
            ui.button(im_str!("##hsplitter1"), (SPLITTER_WIDTH, -1.0));
            let splitter_active = ui.is_item_active();
            if ui.is_item_hovered() || splitter_active {
                ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeEW);
            }
            if let Some(ref mut w) = self.left_pane_size {
                if splitter_active {
                    let mouse_delta_x = ui.imgui().mouse_delta().0;
                    *w += mouse_delta_x;
                }
                let style = ui.imgui().style();
                let minw = style.window_padding.x + style.frame_padding.x;
                let maxw = minw + window_size.0 - SPLITTER_WIDTH - style.window_min_size.x;
                if *w > maxw {
                    *w = maxw;
                } else if *w < minw {
                    *w = minw;
                }
            }
        });
        ui.same_line(0.0);
    }

    fn show_node_list(&mut self, ui: &Ui, dst: &DST<'static, T, E>) {
        const SCROLL_OVER_NODE_OFFSET: Vec2 = Vec2(-50.0, -50.0);

        for (idx, node) in dst.nodes_iter() {
            ui.push_id(idx.id());
            let selected = self.node_states.get_state(&idx, |state| state.selected);
            let name = ImString::new(node.name(&idx));
            if ui.selectable(&name, selected, ImGuiSelectableFlags::empty(), (0.0, 0.0)) {
                if !ui.imgui().key_ctrl() {
                    self.node_states.deselect_all();
                }
                self.node_states.toggle_select(&idx);
                self.active_node = Some(idx);
                self.scrolling.set_target(
                    self.node_states.get_state(&idx, |s| s.pos) + SCROLL_OVER_NODE_OFFSET,
                )
            }
            ui.pop_id();
        }
    }

    fn render_graph_node<ED>(
        &mut self,
        ui: &Ui,
        dst: &DST<'static, T, E>,
        addable_nodes: &[&'static Transform<T, E>],
        addable_macros: &cake::macros::MacroManager<'static, T, E>,
        constant_editor: &ED,
    ) where
        ED: ConstantEditor<T>,
    {
        const EDITOR_EXPORT_FILE: &str = "editor_graph_export.ron";

        ui.child_frame(im_str!("GraphNodeChildWindow"), (0.0, 0.0))
            .build(|| {
                if self.show_top_pane {
                    const TOP_PANE_DESIGN: [StyleVar; 2] = [
                        StyleVar::ItemSpacing(ImVec2 { x: 0.0, y: 0.0 }),
                        StyleVar::ItemInnerSpacing(ImVec2 { x: 0.0, y: 0.0 }),
                    ];
                    ui.with_style_vars(&TOP_PANE_DESIGN, || {
                        ui.checkbox(
                            im_str!("Show connection names."),
                            &mut self.show_connection_names,
                        );
                        ui.same_line_spacing(0.0, 15.0);
                        ui.text(im_str!("Scroll with Ctrl+LMB or Alt+LMB."));
                        ui.same_line(ui.get_window_size().0 - 240.0);
                        if ui.button(im_str!("Import"), (0.0, 0.0)) {
                            self.events.push(RenderEvent::Import);
                        }
                        if ui.is_item_hovered() {
                            ui.tooltip(|| {
                                ui.text(im_str!("Import editor from '{}'.", EDITOR_EXPORT_FILE));
                            });
                        }
                        ui.same_line(ui.get_window_size().0 - 180.0);
                        if ui.button(im_str!("Export"), (0.0, 0.0)) {
                            self.events.push(RenderEvent::Export);
                        }
                        if ui.is_item_hovered() {
                            ui.tooltip(|| {
                                ui.text(im_str!(
                                    "Export editor content to '{}'.",
                                    EDITOR_EXPORT_FILE
                                ));
                            });
                        }
                        ui.same_line(ui.get_window_size().0 - 120.0);
                        ui.checkbox(im_str!("Show grid"), &mut self.show_grid);
                        ui.text(im_str!(
                            "Press Delete or Backspace key to remove selected nodes."
                        ));
                    });
                }
                const GRAPH_STYLE_VAR: [StyleVar; 1] =
                    [StyleVar::FramePadding(ImVec2 { x: 1.0, y: 1.0 })];
                const GRAPH_STYLE_COLOR: [(ImGuiCol, (f32, f32, f32, f32)); 1] =
                    [(ImGuiCol::ChildBg, (0.24, 0.24, 0.27, 0.78))];
                ui.with_style_and_color_vars(&GRAPH_STYLE_VAR, &GRAPH_STYLE_COLOR, || {
                    ui.child_frame(im_str!("scrolling_region"), (0.0, 0.0))
                        .show_borders(true)
                        .show_scrollbar(false)
                        .movable(false)
                        .show_scrollbar_with_mouse(false)
                        .build(|| {
                            // TODO: Manage scaling (and font-scaling)
                            self.render_graph_canvas(
                                ui,
                                dst,
                                addable_nodes,
                                addable_macros,
                                constant_editor,
                            );
                        });
                });
            });
    }

    fn render_graph_canvas<ED>(
        &mut self,
        ui: &Ui,
        dst: &DST<'static, T, E>,
        addable_nodes: &[&'static Transform<T, E>],
        addable_macros: &cake::macros::MacroManager<'static, T, E>,
        constant_editor: &ED,
    ) where
        ED: ConstantEditor<T>,
    {
        const NODE_SLOT_RADIUS: f32 = 6.0 * CURRENT_FONT_WINDOW_SCALE;
        const NODE_CLICK_BOX_RADIUS: f32 = 1.3 * NODE_SLOT_RADIUS;
        const NODE_CLICK_BOX_RADIUS_SQUARED: f32 = NODE_CLICK_BOX_RADIUS * NODE_CLICK_BOX_RADIUS;
        // We don't detect "mouse release" events while dragging links onto slots.
        // Instead we check that our mouse delta is small enough. Otherwise we couldn't
        // hover other slots while dragging links.
        const BASE_NODE_WIDTH: f32 = 120.0 * CURRENT_FONT_WINDOW_SCALE;
        ui.with_item_width(BASE_NODE_WIDTH, || {
            let draw_list = ui.get_window_draw_list();
            draw_list.channels_split(5, |channels| {
                let canvas_size = Vec2::new(ui.get_window_size());
                let win_pos = Vec2::new(ui.get_cursor_screen_pos());
                let offset = win_pos - self.scrolling.get_current();

                if self.show_grid {
                    let cursor_pos = Vec2::new(ui.get_cursor_pos());
                    let offset2 = cursor_pos - self.scrolling.get_current();
                    const GRID_COLOR: [f32; 4] = [0.78, 0.78, 0.78, 0.16];
                    const GRID_SIZE: f32 = 64.0;
                    const GRID_LINE_WIDTH: f32 = 1.0;
                    let grid_sz = CURRENT_FONT_WINDOW_SCALE * GRID_SIZE;
                    let grid_line_width = CURRENT_FONT_WINDOW_SCALE * GRID_LINE_WIDTH;
                    let mut x = offset2.0 % grid_sz;
                    while x < canvas_size.0 {
                        let p1 = Vec2::new((x + win_pos.0, win_pos.1));
                        let p2 = (x + win_pos.0, canvas_size.1 + win_pos.1);
                        draw_list
                            .add_line(p1, p2, GRID_COLOR)
                            .thickness(grid_line_width)
                            .build();
                        x += grid_sz;
                    }
                    let mut y = offset2.1 % grid_sz;
                    while y < canvas_size.1 {
                        let p1 = (win_pos.0, y + win_pos.1);
                        let p2 = (canvas_size.0 + win_pos.0, y + win_pos.1);
                        draw_list
                            .add_line(p1, p2, GRID_COLOR)
                            .thickness(grid_line_width)
                            .build();
                        y += grid_sz;
                    }
                }
                if ui.is_window_hovered() {
                    // Create new node with a popup
                    if ui.imgui().is_mouse_clicked(ImMouseButton::Right) {
                        let mouse_pos = ui.imgui().mouse_pos();
                        if win_pos.0 < mouse_pos.0
                            && mouse_pos.0 < win_pos.0 + canvas_size.0
                            && win_pos.1 < mouse_pos.1
                            && mouse_pos.1 < win_pos.1 + canvas_size.1
                        {
                            ui.open_popup(im_str!("add-new-node"));
                        }
                    }
                    // Scroll
                    if self.drag_node.is_none()
                        && self.creating_link.is_none()
                        && (ui.imgui().key_ctrl() || ui.imgui().key_alt())
                        && ui.imgui().is_mouse_dragging(ImMouseButton::Left)
                    {
                        ui.imgui().set_mouse_cursor(ImGuiMouseCursor::ResizeAll);
                        let delta = Vec2(0.0, 0.0) - ui.imgui().mouse_delta().into();
                        self.scrolling.set_delta(delta);
                    }
                }

                // Bezier control point of the links
                const LINK_CONTROL_POINT_DISTANCE: f32 = 50.0;
                let link_cp =
                    Vec2::new((LINK_CONTROL_POINT_DISTANCE * CURRENT_FONT_WINDOW_SCALE, 0.0));
                const LINK_LINE_WIDTH: f32 = 3.0;
                let link_line_width = LINK_LINE_WIDTH * CURRENT_FONT_WINDOW_SCALE;
                // NODE LINK CULLING?

                for idx in dst.node_ids() {
                    let node_pos = self
                        .node_states
                        .get_state(&idx, |state| state.get_pos(CURRENT_FONT_WINDOW_SCALE));
                    ui.push_id(idx.id());

                    // Display node contents first in the foreground
                    channels.set_current(if self.active_node == Some(idx) { 4 } else { 2 });

                    let node_rect_min = offset + node_pos;
                    let node_rect_max = self
                        .node_states
                        .get_state(&idx, |state| node_rect_min + state.size);
                    ui.set_cursor_screen_pos(node_rect_min + NODE_WINDOW_PADDING);
                    self.draw_node_inside(ui, dst, &draw_list, &idx, constant_editor);

                    let node = dst.get_node(&idx).unwrap();
                    let node_states = &mut self.node_states;
                    let item_rect_size = Vec2::new(ui.get_item_rect_size());
                    node_states.set_state(&idx, |state| {
                        state.size = item_rect_size + NODE_WINDOW_PADDING * 2.0;
                    });

                    channels.set_current(if self.active_node == Some(idx) { 3 } else { 1 });
                    ui.set_cursor_screen_pos(node_rect_min);
                    ui.invisible_button(
                        im_str!("node##nodeinvbtn"),
                        node_states.get_state(&idx, |state| state.size),
                    );
                    // TODO: Handle selection

                    const NODE_ROUNDING: f32 = 4.0;
                    const NODE_COLOR: [f32; 3] = [0.24, 0.24, 0.24];
                    let node_bg_color = NODE_COLOR;
                    draw_list
                        .add_rect(node_rect_min, node_rect_max, node_bg_color)
                        .rounding(NODE_ROUNDING)
                        .filled(true)
                        .build();

                    // Display frame
                    let line_thickness = if node_states.get_state(&idx, |s| s.selected) {
                        3.0
                    } else {
                        1.0
                    } * CURRENT_FONT_WINDOW_SCALE;
                    draw_list
                        .add_rect(node_rect_min, node_rect_max, NODE_FRAME_COLOR)
                        .thickness(line_thickness)
                        .rounding(NODE_ROUNDING)
                        .build();

                    // Display connectors
                    const CONNECTOR_BORDER_THICKNESS: f32 = NODE_SLOT_RADIUS * 0.25;
                    const INPUT_SLOT_COLOR: [f32; 4] = [0.59, 0.59, 0.59, 0.59];
                    for (slot_idx, slot_name) in
                        node.input_slot_names_iter().into_iter().enumerate()
                    {
                        let connector_pos = Vec2::new(node_states.get_state(&idx, |state| {
                            state.get_input_slot_pos(
                                slot_idx,
                                node.inputs_count(),
                                CURRENT_FONT_WINDOW_SCALE,
                            )
                        }));
                        let connector_screen_pos = offset + connector_pos;
                        draw_list
                            .add_circle(connector_screen_pos, NODE_SLOT_RADIUS, INPUT_SLOT_COLOR)
                            .thickness(CONNECTOR_BORDER_THICKNESS)
                            .filled(true)
                            .build();
                        if self.show_connection_names {
                            let slot_name = ImString::new(slot_name);
                            let name_size = ui.calc_text_size(&slot_name, false, -1.0);
                            ui.set_cursor_screen_pos((
                                connector_screen_pos.0 - NODE_SLOT_RADIUS - name_size.x,
                                connector_screen_pos.1 - name_size.y,
                            ));
                            ui.text(slot_name);
                        }
                        if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                            let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                            if (mouse_pos - connector_screen_pos).squared_norm()
                                <= NODE_CLICK_BOX_RADIUS_SQUARED
                            {
                                self.drag_node = None;
                                self.creating_link = Some(LinkExtremity::Input(match idx {
                                    cake::NodeId::Transform(t_idx) => {
                                        InputSlot::Transform(cake::Input::new(t_idx, slot_idx))
                                    }
                                    cake::NodeId::Output(output_id) => InputSlot::Output(output_id),
                                }));
                            }
                        }
                        if let Some(LinkExtremity::Output(link_output)) = self.creating_link {
                            // Check if we hover slot!
                            let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                            if (mouse_pos - connector_screen_pos).squared_norm()
                                <= NODE_CLICK_BOX_RADIUS_SQUARED
                            {
                                self.new_link = Some((
                                    link_output,
                                    match idx {
                                        cake::NodeId::Transform(t_idx) => {
                                            InputSlot::Transform(cake::Input::new(t_idx, slot_idx))
                                        }
                                        cake::NodeId::Output(output_id) => {
                                            InputSlot::Output(output_id)
                                        }
                                    },
                                ));
                                self.creating_link = None;
                            }
                        }
                    }

                    // Show outputs for transform nodes
                    if let cake::NodeId::Transform(t_idx) = idx {
                        const OUTPUT_SLOT_COLOR: [f32; 4] = [0.59, 0.59, 0.59, 0.59];
                        for (slot_idx, type_id) in node.outputs_iter().into_iter().enumerate() {
                            let slot_name = type_id.name();
                            let connector_pos = node_states.get_state(&idx, |state| {
                                state.get_output_slot_pos(
                                    slot_idx,
                                    node.outputs_count(),
                                    CURRENT_FONT_WINDOW_SCALE,
                                )
                            });
                            let connector_screen_pos = offset + connector_pos;
                            draw_list
                                .add_circle(
                                    connector_screen_pos,
                                    NODE_SLOT_RADIUS,
                                    OUTPUT_SLOT_COLOR,
                                )
                                .thickness(CONNECTOR_BORDER_THICKNESS)
                                .filled(true)
                                .build();
                            if self.show_connection_names {
                                let name_size =
                                    ui.calc_text_size(&ImString::new(slot_name), false, -1.0);
                                ui.set_cursor_screen_pos((
                                    connector_screen_pos.0 + NODE_SLOT_RADIUS,
                                    connector_screen_pos.1 - name_size.y,
                                ));
                                ui.text(&ImString::new(slot_name));
                            }
                            if ui.imgui().is_mouse_clicked(ImMouseButton::Left) {
                                let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                                if (mouse_pos - connector_screen_pos).squared_norm()
                                    <= NODE_CLICK_BOX_RADIUS_SQUARED
                                {
                                    self.drag_node = None;
                                    self.creating_link = Some(LinkExtremity::Output(
                                        cake::Output::new(t_idx, slot_idx),
                                    ));
                                }
                            }
                            if let Some(LinkExtremity::Input(link_input)) = self.creating_link {
                                // Check if we hover slot!
                                let mouse_pos: Vec2 = ui.imgui().mouse_pos().into();
                                if (mouse_pos - connector_screen_pos).squared_norm()
                                    <= NODE_CLICK_BOX_RADIUS_SQUARED
                                {
                                    self.new_link =
                                        Some((cake::Output::new(t_idx, slot_idx), link_input));
                                    self.creating_link = None;
                                }
                            }
                        }
                    }
                    ui.pop_id();
                }
                // Preview new link
                const NEW_LINK_COLOR: [f32; 3] = [0.78, 0.78, 0.39];
                if let Some(ref creating_link) = self.creating_link {
                    if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                        let (p1, cp1, cp2, p2) = match *creating_link {
                            LinkExtremity::Output(output) => {
                                let output_node_count =
                                    dst.get_transform(output.t_idx).unwrap().outputs().len();
                                let output_node_state = self
                                    .node_states
                                    .get(&cake::NodeId::Transform(output.t_idx))
                                    .unwrap();
                                let connector_pos = output_node_state.get_output_slot_pos(
                                    output.index(),
                                    output_node_count,
                                    CURRENT_FONT_WINDOW_SCALE,
                                );
                                let p1 = offset + connector_pos;
                                let p2: Vec2 = ui.imgui().mouse_pos().into();
                                let cp1 = p1 + link_cp;
                                let cp2 = p2 - link_cp;
                                (p1, cp1, cp2, p2)
                            }
                            LinkExtremity::Input(input_slot) => {
                                let connector_pos = match input_slot {
                                    InputSlot::Transform(input) => {
                                        let input_node_count = dst
                                            .get_transform(input.t_idx)
                                            .unwrap()
                                            .input_types()
                                            .len();
                                        let input_node_state = self
                                            .node_states
                                            .get(&cake::NodeId::Transform(input.t_idx))
                                            .unwrap();
                                        input_node_state.get_input_slot_pos(
                                            input.index(),
                                            input_node_count,
                                            CURRENT_FONT_WINDOW_SCALE,
                                        )
                                    }
                                    InputSlot::Output(output_id) => {
                                        let input_node_state = self
                                            .node_states
                                            .get(&cake::NodeId::Output(output_id))
                                            .unwrap();
                                        input_node_state.get_input_slot_pos(
                                            0usize,
                                            1usize,
                                            CURRENT_FONT_WINDOW_SCALE,
                                        )
                                    }
                                };
                                let p1 = offset + connector_pos;
                                let p2: Vec2 = ui.imgui().mouse_pos().into();
                                let cp1 = p1 - link_cp;
                                let cp2 = p2 + link_cp;
                                (p1, cp1, cp2, p2)
                            }
                        };
                        draw_list
                            .add_bezier_curve(p1, cp1, cp2, p2, NEW_LINK_COLOR)
                            .thickness(link_line_width)
                            .build();
                    }
                }
                if self.creating_link.is_some() && !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                    self.creating_link = None;
                }

                // Display links
                channels.set_current(0);
                for (output, input_slot) in dst.links_iter() {
                    let connector_in_pos = match input_slot {
                        cake::InputSlot::Transform(input) => {
                            let input_node_count =
                                dst.get_transform(input.t_idx).unwrap().input_types().len();
                            let input_node_state = self
                                .node_states
                                .get(&cake::NodeId::Transform(input.t_idx))
                                .unwrap();
                            input_node_state.get_input_slot_pos(
                                input.index(),
                                input_node_count,
                                CURRENT_FONT_WINDOW_SCALE,
                            )
                        }
                        cake::InputSlot::Output(output_id) => {
                            let input_node_state = self
                                .node_states
                                .get(&cake::NodeId::Output(output_id))
                                .unwrap();
                            input_node_state.get_input_slot_pos(
                                0usize,
                                1usize,
                                CURRENT_FONT_WINDOW_SCALE,
                            )
                        }
                    };
                    let p1 = offset + connector_in_pos;
                    let output_node_count =
                        dst.get_transform(output.t_idx).unwrap().outputs().len();
                    let output_node_state = self
                        .node_states
                        .get(&cake::NodeId::Transform(output.t_idx))
                        .unwrap();

                    let connector_out_pos = output_node_state.get_output_slot_pos(
                        output.index(),
                        output_node_count,
                        CURRENT_FONT_WINDOW_SCALE,
                    );
                    let p2 = offset + connector_out_pos;
                    let cp1 = p1 - link_cp;
                    let cp2 = p2 + link_cp;
                    const LINK_COLOR: [f32; 3] = [0.78, 0.78, 0.39];
                    draw_list
                        .add_bezier_curve(p1, cp1, cp2, p2, LINK_COLOR)
                        .thickness(link_line_width)
                        .build();
                }
            })
        });
        if let Some((output, input_slot)) = self.new_link {
            self.events.push(RenderEvent::Connect(output, input_slot));
            self.new_link = None;
        }
        ui.popup(im_str!("add-new-node"), || {
            ui.text("Add node");
            ui.separator();
            for (i, node) in addable_nodes.iter().enumerate() {
                ui.push_id(i as i32);
                if ui.menu_item(&ImString::new(node.name())).build() {
                    self.events.push(RenderEvent::AddTransform(node));
                }
                ui.pop_id();
            }
            ui.separator();
            if ui.menu_item(im_str!("New macro")).build() {
                self.events.push(RenderEvent::AddNewMacro);
            }
            for macr in addable_macros.macros() {
                ui.with_id(macr.id() as i32, || {
                    if ui.menu_item(&ImString::new(macr.name())).build() {
                        self.events.push(RenderEvent::AddMacro(macr.clone()));
                    }
                });
            }
            ui.separator();
            if ui.menu_item(im_str!("Output node")).build() {
                self.events.push(RenderEvent::CreateOutput);
            }
            ui.separator();
            for constant_type in T::editable_variants() {
                let item_name = ImString::new(format!("Input node: {}", constant_type));
                if ui.menu_item(&item_name).build() {
                    self.events.push(RenderEvent::AddConstant(constant_type));
                }
            }
        });
    }

    fn draw_node_inside<ED>(
        &mut self,
        ui: &Ui,
        dst: &DST<'static, T, E>,
        draw_list: &WindowDrawList,
        id: &cake::NodeId,
        constant_editor: &ED,
    ) where
        ED: ConstantEditor<T>,
    {
        let (node_name, description) = {
            let node = dst.get_node(id).unwrap();
            (
                ImString::new(node.name(id)),
                ImString::new(match node {
                    cake::Node::Transform(t) => t.description(),
                    cake::Node::Output(_) => format!(
                        "Visualize the data that flows into this node\nin the window '{}'",
                        node.name(id),
                    )
                    .into(),
                }),
            )
        };
        let node_states = &mut self.node_states;
        let events = &mut self.events;
        let mut title_bar_height = 0.0;
        let p = ui.get_cursor_screen_pos();

        ui.group(|| {
            let default_text_color = ui.imgui().style().colors[ImGuiCol::Text as usize];
            ui.with_color_var(ImGuiCol::Text, default_text_color, || {
                ui.text(&node_name);
                title_bar_height = ui.get_item_rect_size().1;
                if ui.is_item_hovered() {
                    ui.tooltip(|| ui.text(description));
                }
            });
            ui.dummy((0.0, NODE_WINDOW_PADDING.1 / 2.0));
            if let cake::NodeId::Transform(t_idx) = *id {
                if let Some(t) = dst.get_transform(t_idx) {
                    if let cake::Algorithm::Constant(ref constant) = t.algorithm() {
                        if let Some(new_value) = constant_editor.editor(ui, &constant, 0, false) {
                            events.push(RenderEvent::SetConstant(t_idx, Box::new(new_value)));
                        }
                    }
                }
                let outputs = dst.outputs_attached_to_transform(t_idx).unwrap();
                if let Some(default_inputs) = dst.get_default_inputs(t_idx) {
                    for (i, (default_input, some_output)) in
                        default_inputs.into_iter().zip(outputs).enumerate()
                    {
                        let read_only = some_output.is_some();
                        if let Some(val) = default_input {
                            if let Some(new_value) =
                                constant_editor.editor(ui, &val, i as i32, read_only)
                            {
                                events.push(RenderEvent::WriteDefaultInput {
                                    t_idx,
                                    input_index: i,
                                    val: Box::new(new_value),
                                })
                            }
                        } else {
                            // Fill with dummy line for vertical alignment
                            ui.text("");
                        }
                    }
                }
            }
            // TODO: Add copy-paste buttons
        });

        // Line below node name
        let node_size = ui.get_item_rect_size();
        let line_thickness = if self.active_node == Some(*id) {
            3.0
        } else {
            1.0
        } * CURRENT_FONT_WINDOW_SCALE;
        draw_list
            .add_line(
                [
                    p.0 - NODE_WINDOW_PADDING.0,
                    p.1 + title_bar_height + NODE_WINDOW_PADDING.1 / 2.0,
                ],
                [
                    p.0 + node_size.0 + NODE_WINDOW_PADDING.0,
                    p.1 + title_bar_height + NODE_WINDOW_PADDING.1 / 2.0,
                ],
                NODE_FRAME_COLOR,
            )
            .thickness(line_thickness)
            .build();

        if ui.is_item_hovered()
            && !ui.is_item_active()
            && ui.imgui().is_mouse_clicked(ImMouseButton::Left)
        {
            self.active_node = Some(*id);
            self.drag_node = Some(*id);
            if !ui.imgui().key_ctrl() {
                node_states.deselect_all();
            }
            node_states.toggle_select(id);
        }
        if self.drag_node == Some(*id) {
            if ui.imgui().is_mouse_dragging(ImMouseButton::Left) {
                let delta = ui.imgui().mouse_delta();
                node_states.set_state(id, |state| {
                    state.pos = state.pos + delta.into();
                });
            } else if !ui.imgui().is_mouse_down(ImMouseButton::Left) {
                self.drag_node = None;
            }
        }

        if ui.is_item_hovered()
            && !ui.is_item_active()
            && ui.imgui().is_mouse_double_clicked(ImMouseButton::Left)
        {
            events.push(RenderEvent::EditNode(*id));
        }
    }
}

impl<T, E> NodeEditorLayout<T, E>
where
    T: VariantName,
{
    fn delete_selected_nodes(&mut self) {
        let selected_node_ids: Vec<_> = self
            .node_states
            .iter()
            .filter(|(_, state)| state.selected)
            .map(|(id, _)| *id)
            .collect();
        for node_id in selected_node_ids {
            self.events.push(RenderEvent::RemoveNode(node_id));
            self.node_states.remove_node(&node_id);
            if self.active_node == Some(node_id) {
                self.active_node.take();
            }
        }
    }
}

impl<T, E> NodeEditorLayout<T, E> {
    pub fn scrolling(&self) -> &Scrolling {
        &self.scrolling
    }

    pub fn node_states(&self) -> &NodeStates {
        &self.node_states
    }

    pub fn import(&mut self, node_states: NodeStates, scrolling: Scrolling) {
        // Set UI node states
        self.node_states = node_states;
        // Set scrolling offset
        self.scrolling = scrolling;

        // Reset all temporary values
        self.active_node = None;
        self.drag_node = None;
        self.creating_link = None;
        self.new_link = None;
    }
}
