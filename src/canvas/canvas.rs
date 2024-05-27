use super::tool::{Pending, Tool};
use super::tools::line::LinePending;
use crate::canvas::layer::{CanvasMessage, Layer, LayerVessel};
use crate::canvas::style::Style;
use crate::canvas::svg::SVG;
use crate::database;
use crate::scene::{Globals, Message};
use crate::utils::serde::Serialize;
use crate::utils::theme::Theme;
use directories::ProjectDirs;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Quad;
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::canvas;
use iced::{Color, Command, Element, Event, Length, Rectangle, Renderer, Size};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{Document, Uuid};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::sync::Arc;
use svg::node::element::Group;

/// The canvas structure.
pub struct Canvas {
    /// The id of the drawing.
    id: Uuid,

    /// The width of the [Canvas].
    width: Length,

    /// The height of the [Canvas].
    height: Length,

    /// The name of the drawing.
    name: String,

    /// Tells whether the name of the drawing is being edited or not.
    new_name: Option<String>,

    /// The ids of layers ordered.
    layer_order: Vec<Uuid>,

    /// The list of caches corresponding to each [Layer].
    layers: Box<HashMap<Uuid, Layer>>,

    /// The index of currently active layer.
    current_layer: Uuid,

    /// A list of all the [tools](Tool).
    tools: Box<Vec<(Arc<dyn Tool>, Uuid)>>,

    /// A list of the removed [tools](Tool).
    undo_stack: Box<Vec<(Arc<dyn Tool>, Uuid)>>,

    /// The index where the [Tool] list was last saved.
    last_saved: usize,

    /// The amount of [tools](Tool) that were saved in total.
    count_saved: usize,

    /// Tells whether the layers layout has been modified.
    edited_layers: bool,

    /// Holds the ids of the removed layers; useful for online updates.
    removed_layers: Vec<Uuid>,

    /// A [SVG] that holds the same drawing; used when making a post.
    svg: SVG,

    /// A list of the tools held in [json](JsonValue) form. Used when the drawing is stored locally.
    json_tools: Option<Vec<JsonValue>>,

    /// The currently selected [Tool].
    current_tool: Box<dyn Pending>,

    /// The [Style] applied to the current [Tool].
    style: Style,
}

impl Canvas {
    /// A default value, will be properly initialized by the [Loaded](CanvasAction::Loaded) message.
    pub fn new() -> Self {
        Canvas {
            id: Uuid::from_bytes([0; 16]),
            width: Length::Fill,
            height: Length::Fill,
            name: String::from(""),
            new_name: None,
            layer_order: vec![],
            layers: Box::new(HashMap::from_iter(vec![])),
            current_layer: Uuid::new(),
            tools: Box::new(vec![]),
            undo_stack: Box::new(vec![]),
            last_saved: 0,
            count_saved: 0,
            edited_layers: false,
            removed_layers: vec![],
            svg: SVG::new(&vec![]),
            json_tools: None,
            current_tool: Box::new(LinePending::None),
            style: Style::default(),
        }
    }

    /// Returns the number of layers.
    pub fn get_layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_new_name(&self) -> &Option<String> {
        &self.new_name
    }

    pub fn get_svg(&self) -> &SVG {
        &self.svg
    }

    pub fn get_style(&self) -> &Style {
        &self.style
    }

    pub fn get_layer_order(&self) -> &Vec<Uuid> {
        &self.layer_order
    }

    pub fn get_current_layer(&self) -> &Uuid {
        &self.current_layer
    }

    pub fn get_layers(&self) -> &Box<HashMap<Uuid, Layer>> {
        &self.layers
    }

    pub fn get_current_tool(&self) -> &Box<dyn Pending> {
        &self.current_tool
    }

    pub fn set_id(&mut self, id: impl Into<Uuid>) {
        self.id = id.into();
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn set_new_name(&mut self, new_name: impl Into<Option<String>>) {
        self.new_name = new_name.into();
    }

    /// Returns the new unsaved tools as mongodb [documents](Document).
    fn get_tools_serialized(&self) -> Vec<Document> {
        let mut vec = vec![];

        for pos in self.count_saved..self.tools.len() {
            let val = self.tools.get(pos);

            if let Some((tool, layer)) = val {
                let mut document: Document = tool.serialize();
                document.insert("order", pos.clone() as u32);
                document.insert("canvas_id", self.id);
                document.insert("name", tool.id());
                document.insert("layer", layer);

                vec.push(document);
            }
        }

        vec
    }

    /// Returns the new unsaved tools as svg [groups](Group).
    fn get_tools_svg(&self) -> Vec<(Group, Uuid)> {
        let mut vec = vec![];

        for pos in self.count_saved..self.tools.len() {
            let val = self.tools.get(pos);

            if let Some((tool, layer)) = val {
                vec.push((
                    Serialize::<Group>::serialize(tool.boxed_clone().deref()),
                    *layer,
                ));
            }
        }

        vec
    }

    /// Returns the new unsaved tools as json [objects](JsonValue).
    fn get_tools_json(&self) -> Vec<JsonValue> {
        let mut vec = vec![];

        for pos in self.count_saved..self.tools.len() {
            let val = self.tools.get(pos);

            if let Some((tool, layer)) = val {
                let mut data: Object = Serialize::<Object>::serialize(tool.boxed_clone().deref());
                data.insert("name", JsonValue::String(tool.id()));
                data.insert("layer", JsonValue::String(layer.to_string()));

                vec.push(JsonValue::Object(data));
            }
        }

        vec
    }

    /// Clears the cache of a layer.
    fn clear_cache(&mut self, layer: Uuid) {
        let layer = self.layers.get(&layer);
        if let Some(layer) = layer {
            layer.clear_cache();
        }
    }

    /// Sets the width of the canvas.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the canvas.
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Update function, all canvas related messages are handled here.
    pub fn update(&mut self, globals: &mut Globals, message: CanvasMessage) -> Command<Message> {
        match message {
            CanvasMessage::ToggleEditName => match self.new_name.clone().as_deref() {
                Some("") => {}
                Some(new_name) => {
                    self.new_name = None;
                    if self.name != new_name {
                        self.name = String::from(new_name);
                        self.edited_layers = true;
                    }
                }
                None => {
                    self.new_name = Some(self.name.clone());
                }
            },
            CanvasMessage::SetNewName(new_name) => {
                if new_name.len() <= 40 {
                    self.new_name = Some(new_name);
                }
            }
            CanvasMessage::UseTool(tool) => {
                self.tools.push((tool.clone(), self.current_layer));
                self.layers
                    .get_mut(&self.current_layer)
                    .unwrap()
                    .get_mut_tools()
                    .push(tool.clone());
                self.undo_stack = Box::new(vec![]);
                self.clear_cache(self.current_layer);
            }
            CanvasMessage::UpdateStyle(update) => {
                return self.style.update(update);
            }
            CanvasMessage::AddLayer => {
                let layer_id = Uuid::new();

                self.svg.add_layer(layer_id);
                self.layer_order.push(layer_id);
                self.layers.insert(layer_id, Default::default());
                self.current_tool = self.current_tool.dyn_default();
                self.current_layer = layer_id;
                self.edited_layers = true;
            }
            CanvasMessage::ActivateLayer(layer) => {
                self.current_tool = self.current_tool.dyn_default();
                self.current_layer = layer;
            }
            CanvasMessage::ToggleLayer(layer) => {
                self.layers.get_mut(&layer).unwrap().toggle_visibility();
            }
            CanvasMessage::ToggleEditLayerName(layer) => {
                self.layers.get_mut(&layer).unwrap().toggle_name();
                self.edited_layers = true;
            }
            CanvasMessage::UpdateLayerName(id, name) => {
                self.layers.get_mut(&id).unwrap().set_new_name(name);
            }
            CanvasMessage::RemoveLayer(id) => {
                if let Some(ref mut json_tools) = self.json_tools {
                    json_tools.retain(|tool| {
                        if let JsonValue::Object(object) = tool {
                            if let Some(json_value) = object.get("id") {
                                if let JsonValue::String(layer_id) = json_value {
                                    return layer_id.clone() != id.to_string();
                                }
                            }
                        }

                        false
                    });
                }

                self.tools.retain(|(_, layer_id)| *layer_id != id);
                self.undo_stack.retain(|(_, layer_id)| *layer_id != id);
                self.layers.remove(&id);
                self.layer_order.retain(|layer_id| *layer_id != id);

                self.edited_layers = true;
                self.removed_layers.push(id);

                if self.current_layer == id {
                    return self.update(globals, CanvasMessage::ActivateLayer(self.layer_order[0]));
                }
            }
            CanvasMessage::Save => {
                let tools_svg = self.get_tools_svg();
                if tools_svg.is_empty()
                    && self.count_saved == self.last_saved
                    && !self.edited_layers
                {
                    return Command::none();
                }

                let delete_lower_bound = self.count_saved;
                let delete_upper_bound = self.last_saved;

                for _ in delete_lower_bound..delete_upper_bound {
                    self.svg.remove();
                }
                for (tool, layer) in tools_svg {
                    self.svg.add_tool(&layer, tool);
                }

                let canvas_id = self.id;
                let layers: Vec<(Uuid, String)> = self
                    .layer_order
                    .iter()
                    .map(|id| (*id, self.layers.get(id).unwrap().get_name().clone()))
                    .collect();

                let canvas_name = self.name.clone();

                return if let Some(mut tools) = self.json_tools.clone() {
                    let tools_json = self.get_tools_json();

                    Command::perform(
                        async move {
                            let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
                            let dir_path = proj_dirs.data_local_dir();

                            let file_path = dir_path.join(canvas_id.to_string()).join("data.json");
                            let drawings_path = dir_path.join("drawings.json");

                            let drawings = fs::read_to_string(drawings_path.clone()).unwrap();
                            let mut drawings = json::parse(&*drawings).unwrap();

                            if let JsonValue::Array(drawings) = &mut drawings {
                                for drawing in drawings {
                                    if let JsonValue::Object(drawing) = drawing {
                                        if let Some(JsonValue::String(id)) = drawing.get("id") {
                                            if id.clone() == canvas_id.to_string() {
                                                drawing
                                                    .insert("name", JsonValue::String(canvas_name));
                                                break;
                                            };
                                        }
                                    }
                                }
                            }

                            fs::write(drawings_path, json::stringify(drawings)).unwrap();

                            for _ in delete_lower_bound..delete_upper_bound {
                                tools.pop();
                            }

                            tools.extend(tools_json);

                            let mut data = Object::new();
                            data.insert(
                                "layers",
                                JsonValue::Array(
                                    layers
                                        .iter()
                                        .map(|(id, name)| {
                                            let mut object = Object::new();
                                            object.insert("id", JsonValue::String(id.to_string()));
                                            object.insert("name", JsonValue::String(name.clone()));

                                            JsonValue::Object(object)
                                        })
                                        .collect(),
                                ),
                            );
                            data.insert("tools", JsonValue::Array(tools));

                            fs::write(file_path, json::stringify(JsonValue::Object(data))).unwrap();
                        },
                        |()| CanvasMessage::Saved.into(),
                    )
                } else {
                    let tools_mongo = self.get_tools_serialized();
                    let removed_layers = self.removed_layers.clone();
                    let layer_data = self
                        .layers
                        .iter()
                        .map(|(id, layer)| (*id, layer.get_name().clone()))
                        .collect::<Vec<(Uuid, String)>>();
                    let db = globals.get_db();

                    if let Some(db) = db {
                        Command::perform(
                            async move {
                                database::drawing::update_drawing(
                                    &db,
                                    canvas_id,
                                    canvas_name.clone(),
                                    delete_lower_bound as u32,
                                    delete_upper_bound as u32,
                                    tools_mongo,
                                    removed_layers,
                                    layer_data,
                                )
                                .await
                            },
                            move |result| match result {
                                Ok(()) => CanvasMessage::Saved.into(),
                                Err(err) => Message::Error(err),
                            },
                        )
                    } else {
                        Command::none()
                    }
                };
            }
            CanvasMessage::Undo => {
                let opt = self.tools.pop();
                if let Some((tool, layer)) = opt {
                    self.layers.get_mut(&layer).unwrap().get_mut_tools().pop();
                    self.undo_stack.push((tool.clone(), layer));

                    self.clear_cache(layer);
                }

                if self.count_saved > self.tools.len() {
                    self.count_saved = self.count_saved - 1;
                }
            }
            CanvasMessage::Redo => {
                let opt = self.undo_stack.pop();

                if let Some((tool, layer)) = opt {
                    self.tools.push((tool.clone(), layer));
                    self.layers
                        .get_mut(&layer)
                        .unwrap()
                        .get_mut_tools()
                        .push(tool.clone());
                    self.clear_cache(layer);
                }
            }
            CanvasMessage::ChangeTool(tool) => {
                self.current_tool = (*tool).boxed_clone();
                self.current_tool.shape_style(&mut self.style);
            }
            CanvasMessage::Saved => {
                self.count_saved = self.tools.len();
                self.last_saved = self.count_saved;
            }
            CanvasMessage::Loaded {
                layers,
                tools,
                json_tools,
            } => {
                self.tools = Box::new(vec![]);
                self.layers = Box::new(HashMap::from_iter(
                    layers
                        .iter()
                        .map(|(id, name)| (*id, Layer::new(name.clone()))),
                ));
                self.layer_order = layers.iter().map(|(id, _)| *id).collect();
                self.svg = SVG::new(&self.layer_order);
                self.current_layer = self.layer_order[0];

                for (tool, layer) in tools {
                    self.tools.push((tool.clone(), layer));
                    self.layers
                        .get_mut(&layer)
                        .unwrap()
                        .get_mut_tools()
                        .push(tool.clone());
                    self.svg.add_tool(
                        &layer,
                        Serialize::<Group>::serialize(tool.boxed_clone().deref()),
                    );
                }

                self.count_saved = self.tools.len();
                self.last_saved = self.count_saved;

                self.json_tools = json_tools;
            }
        }
        Command::none()
    }
}

impl<'a> From<&'a Canvas> for Element<'a, Message, Theme, Renderer> {
    fn from(value: &'a Canvas) -> Self {
        Element::new(CanvasVessel::new(value)).map(Into::into)
    }
}

/// A struct that holds the [canvas](canvas::Canvas) objects for each layer, and handles the interaction.
struct CanvasVessel<'a> {
    /// The width of the [Canvas].
    width: Length,

    /// The height of the [Canvas].
    height: Length,

    /// The order of the layers.
    layer_order: &'a [Uuid],

    /// The list of data for each [Layer].
    states: &'a HashMap<Uuid, Layer>,

    /// The list of [canvas layers](Canvas).
    layers: HashMap<Uuid, canvas::Canvas<LayerVessel<'a>, CanvasMessage, Theme, Renderer>>,

    /// The index of the currently active [Layer].
    current_layer: Uuid,
}

impl<'a> CanvasVessel<'a> {
    /// Creates a new [Canvas] widget.
    fn new(canvas: &'a Canvas) -> Self {
        let mut vessel = CanvasVessel {
            width: canvas.width,
            height: canvas.height,
            states: &canvas.layers,
            layer_order: &canvas.layer_order,
            layers: HashMap::new(),
            current_layer: canvas.current_layer,
        };

        vessel.layers = HashMap::from_iter(vessel.states.iter().map(|(pos, state)| {
            (
                *pos,
                canvas::Canvas::new(LayerVessel::new(
                    state.get_cache(),
                    state.get_tools(),
                    &canvas.current_tool,
                    &canvas.style,
                    *pos == vessel.current_layer,
                )),
            )
        }));

        vessel
    }
}

impl<'a> Widget<CanvasMessage, Theme, Renderer> for CanvasVessel<'a> {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        if self.layer_order.len() == 0 {
            return Node::default();
        }

        let limits = limits.loose().width(self.width).height(self.height);
        let mut nodes = vec![];

        for (index, layer) in (0..self.layers.len()).zip(self.layer_order) {
            nodes.push(self.layers[&layer].layout(&mut tree.children[index], renderer, &limits));
        }

        Node::with_children(nodes[0].size(), nodes)
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let mut children = layout.children();
        let bounds = layout.bounds();

        iced::advanced::Renderer::fill_quad(
            renderer,
            Quad {
                bounds,
                border: Default::default(),
                shadow: Default::default(),
            },
            Color::WHITE,
        );

        for (layer, index) in self.layer_order.iter().zip(0..self.layers.len()) {
            if self.states.get(&layer).unwrap().is_visible() {
                self.layers[&layer].draw(
                    &state.children[index],
                    renderer,
                    theme,
                    style,
                    children
                        .next()
                        .expect(&*format!("Canvas needs to have at least {} layers.", index)),
                    cursor,
                    viewport,
                );
            }
        }
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<
            Tag<<LayerVessel<'_> as canvas::Program<CanvasMessage, Theme, Renderer>>::State>,
        >()
    }

    fn state(&self) -> tree::State {
        tree::State::new(<LayerVessel<'_> as canvas::Program<
            CanvasMessage,
            Theme,
            Renderer,
        >>::State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.layer_order
            .iter()
            .map(|layer| {
                Tree::new(&self.layers[&layer] as &dyn Widget<CanvasMessage, Theme, Renderer>)
            })
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(
            self.layer_order
                .iter()
                .map(|layer| &self.layers[&layer] as &dyn Widget<CanvasMessage, Theme, Renderer>)
                .collect::<Vec<&dyn Widget<CanvasMessage, Theme, Renderer>>>()
                .as_slice(),
        )
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, CanvasMessage>,
        viewport: &Rectangle,
    ) -> Status {
        if self.layer_order.len() == 0 {
            return Status::Ignored;
        }

        let layer = self.layers.get_mut(&self.current_layer).unwrap();
        let mut children = layout.children();
        let binding = Node::default();
        let mut layout = Layout::new(&binding);
        let mut index = 0;

        for id in self.layer_order {
            layout = children.next().expect(&*format!(
                "Canvas needs to have at least {} children.",
                self.current_layer
            ));
            if *id == self.current_layer {
                break;
            }
            index += 1;
        }

        layer.on_event(
            &mut state.children[index],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        if self.layer_order.len() == 0 {
            return Interaction::default();
        }

        let mut children = layout.children();
        let binding = Node::default();
        let mut layout = Layout::new(&binding);
        let mut index = 0;

        for id in self.layer_order {
            layout = children.next().expect(&*format!(
                "Canvas needs to have at least {} children.",
                self.current_layer
            ));
            if *id == self.current_layer {
                break;
            }
            index += 1;
        }

        self.layers[&self.current_layer].mouse_interaction(
            &state.children[index],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}
