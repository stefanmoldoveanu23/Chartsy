use super::tool::{Pending, Tool};
use super::tools::line::LinePending;
use crate::canvas::layer::{CanvasAction, Layer};
use crate::canvas::style::Style;
use crate::canvas::svg::SVG;
use crate::mongo::{MongoRequest, MongoRequestType};
use crate::scene::{Globals, Message};
use crate::scenes::drawing::DrawingAction;
use crate::serde::Serialize;
use crate::theme::Theme;
use directories::ProjectDirs;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::canvas;
use iced::{Command, Element, Event, Length, Rectangle, Renderer, Size};
use json::object::Object;
use json::JsonValue;
use mongodb::bson::{doc, Document, Uuid};
use std::fs::OpenOptions;
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;
use svg::node::element::Group;

/// Holds the [cache](canvas::Cache) for a canvas layer.
pub struct State {
    cache: canvas::Cache,
}

/// The canvas structure.
pub struct Canvas {
    /// The id of the drawing.
    pub(crate) id: Uuid,

    /// The width of the [Canvas].
    pub(crate) width: Length,

    /// The height of the [Canvas].
    pub(crate) height: Length,

    /// The list of caches corresponding to each [Layer].
    pub(crate) layers: Box<Vec<State>>,

    /// The index of currently active layer.
    pub(crate) current_layer: usize,

    /// A list of all the [tools](Tool).
    pub(crate) tools: Box<Vec<(Arc<dyn Tool>, usize)>>,

    /// A list of the removed [tools](Tool).
    pub(crate) undo_stack: Box<Vec<(Arc<dyn Tool>, usize)>>,

    /// The added [tools](Tool) organized by [Layer].
    pub(crate) tool_layers: Box<Vec<Vec<Arc<dyn Tool>>>>,

    /// The index where the [Tool] list was last saved.
    pub(crate) last_saved: usize,

    /// The amount of [tools](Tool) that were saved in total.
    pub(crate) count_saved: usize,

    /// A [SVG] that holds the same drawing; used when making a post.
    pub(crate) svg: SVG,

    /// A list of the tools held in [json](JsonValue) form. Used when the drawing is stored locally.
    pub(crate) json_tools: Option<Vec<JsonValue>>,

    /// The automatically selected [Tool] when the drawing is opened.
    pub(crate) default_tool: Box<dyn Pending>,

    /// The currently selected [Tool].
    pub(crate) current_tool: Box<dyn Pending>,

    /// The [Style] applied to the current [Tool].
    pub(crate) style: Style,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl Canvas {
    /// A default value, will be properly initialized by the [Loaded](CanvasAction::Loaded) message.
    pub(crate) fn new() -> Self {
        Canvas {
            id: Uuid::from_bytes([0; 16]),
            width: Length::Fill,
            height: Length::Fill,
            layers: Box::new(vec![State {
                cache: canvas::Cache::default(),
            }]),
            current_layer: 0,
            tools: Box::new(vec![]),
            undo_stack: Box::new(vec![]),
            tool_layers: Box::new(vec![vec![]]),
            last_saved: 0,
            count_saved: 0,
            svg: SVG::new(0),
            json_tools: None,
            default_tool: Box::new(LinePending::None),
            current_tool: Box::new(LinePending::None),
            style: Style::default(),
        }
    }

    /// Returns the number of layers.
    pub fn get_layer_count(&self) -> usize {
        self.layers.len()
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
                document.insert("layer", *layer as u32);

                vec.push(document);
            }
        }

        vec
    }

    /// Returns the new unsaved tools as svg [groups](Group).
    fn get_tools_svg(&self) -> Vec<(Group, usize)> {
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
                data.insert("layer", JsonValue::Number((*layer as f32).into()));

                vec.push(JsonValue::Object(data));
            }
        }

        vec
    }

    /// Clears the cache of a layer.
    fn clear_cache(&mut self, layer: usize) {
        let layer = self.layers.get(layer);
        if let Some(state) = layer {
            state.cache.clear();
        }
    }

    /// Sets the width of the canvas.
    pub(crate) fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the canvas.
    pub(crate) fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Update function, all canvas related messages are handled here.
    pub(crate) fn update(&mut self, globals: &mut Globals, message: CanvasAction) -> Command<Message> {
        match message {
            CanvasAction::UseTool(tool) => {
                self.tools.push((tool.clone(), self.current_layer));
                self.tool_layers[self.current_layer].push(tool.clone());
                self.undo_stack = Box::new(vec![]);
                self.clear_cache(self.current_layer);
            }
            CanvasAction::UpdateStyle(update) => {
                return self.style.update(update);
            }
            CanvasAction::AddLayer => {
                self.tool_layers.push(vec![]);
                self.layers.push(State {
                    cache: canvas::Cache::default(),
                });
                self.current_tool = self.default_tool.clone();
                self.current_layer = self.layers.len() - 1;
                self.svg.add_layer();

                if self.json_tools.is_none() {
                    let id = self.id.clone();
                    let layers = self.layers.len();
                    let db = globals.get_db();

                    if let Some(db) = db {
                        return Command::perform(
                            async move {
                                MongoRequest::send_requests(
                                    db,
                                    vec![
                                        MongoRequest::new(
                                            "canvases".into(),
                                            MongoRequestType::Update {
                                                filter: doc! { "id": id },
                                                update: doc! { "$set": { "layers": layers as u32 } },
                                                options: None
                                            }
                                        )
                                    ]
                                ).await
                            },
                            |responses| {
                                match responses {
                                    Ok(_) => {
                                        Message::DoAction(Box::new(DrawingAction::None))
                                    }
                                    Err(message) => {
                                        message
                                    }
                                }
                            }
                        );
                    }
                }
            }
            CanvasAction::ActivateLayer(layer) => {
                self.current_tool = self.default_tool.clone();
                self.current_layer = layer;
            }
            CanvasAction::Save => {
                let tools_svg = self.get_tools_svg();
                if tools_svg.is_empty() && self.count_saved == self.last_saved {
                    return Command::none();
                }

                let delete_lower_bound = self.count_saved;
                let delete_upper_bound = self.last_saved;

                for _ in delete_lower_bound..delete_upper_bound {
                    self.svg.remove();
                }
                for (tool, layer) in tools_svg {
                    self.svg.add_tool(layer, tool);
                }

                let canvas_id = self.id;
                let layers = self.layers.len();

                return if let Some(mut tools) = self.json_tools.clone() {
                    let tools_json = self.get_tools_json();

                    Command::perform(
                        async move {
                            let proj_dirs = ProjectDirs::from("", "CharMe", "Chartsy").unwrap();
                            let file_path = proj_dirs
                                .data_local_dir()
                                .join(String::from("./") + &*canvas_id.to_string() + "/data.json");
                            let mut file = OpenOptions::new().write(true).open(file_path).unwrap();

                            for _ in delete_lower_bound..delete_upper_bound {
                                tools.pop();
                            }

                            tools.extend(tools_json);

                            let mut data = Object::new();
                            data.insert("layers", JsonValue::Number(layers.into()));
                            data.insert("tools", JsonValue::Array(tools));

                            file.write(json::stringify(JsonValue::Object(data)).as_ref())
                                .unwrap();
                        },
                        |()| {
                            Message::DoAction(Box::new(DrawingAction::CanvasAction(
                                CanvasAction::Saved,
                            )))
                        },
                    )
                } else {
                    let tools_mongo = self.get_tools_serialized();
                    let db = globals.get_db();

                    if let Some(db) = db {
                        Command::perform(
                        async move {
                            MongoRequest::send_requests(
                                db,
                                vec![
                                    MongoRequest::new(
                                        "tools".into(),
                                        MongoRequestType::Delete{
                                            filter: doc! {
                                                "canvas_id": canvas_id,
                                                "order": {
                                                    "$gte": delete_lower_bound as u32,
                                                    "$lte": delete_upper_bound as u32,
                                                }
                                            },
                                            options: None
                                        },
                                    ),
                                    MongoRequest::new(
                                        "tools".into(),
                                        MongoRequestType::Insert{
                                            documents: tools_mongo,
                                            options: None,
                                        },
                                    ),
                                ]
                            ).await
                        },
                        move |_| {
                            Message::DoAction(Box::new(DrawingAction::CanvasAction(CanvasAction::Saved)))
                        })
                    } else {
                        Command::none()
                    }
                };
            }
            CanvasAction::Undo => {
                let opt = self.tools.pop();
                if let Some((tool, layer)) = opt {
                    self.tool_layers[layer].pop();
                    self.undo_stack.push((tool.clone(), layer));

                    self.clear_cache(layer);
                }

                if self.count_saved > self.tools.len() {
                    self.count_saved = self.count_saved - 1;
                }
            }
            CanvasAction::Redo => {
                let opt = self.undo_stack.pop();

                if let Some((tool, layer)) = opt {
                    self.tools.push((tool.clone(), layer));
                    self.tool_layers[layer].push(tool.clone());
                    self.clear_cache(layer);
                }
            }
            CanvasAction::ChangeTool(tool) => {
                self.current_tool = (*tool).boxed_clone();
                self.default_tool = (*tool).boxed_clone();
                self.current_tool.shape_style(&mut self.style);
            }
            CanvasAction::Saved => {
                self.count_saved = self.tools.len();
                self.last_saved = self.count_saved;
            }
            CanvasAction::Loaded {
                layers,
                tools,
                json_tools,
            } => {
                self.tools = Box::new(vec![]);
                self.tool_layers = Box::new(vec![vec![]; layers]);
                self.layers = Box::new(vec![]);
                self.svg = SVG::new(layers);

                for (tool, layer) in tools {
                    self.tools.push((tool.clone(), layer));
                    self.tool_layers[layer].push(tool.clone());
                    self.svg.add_tool(
                        layer,
                        Serialize::<Group>::serialize(tool.boxed_clone().deref()),
                    )
                }

                self.count_saved = self.tools.len();
                self.last_saved = self.count_saved;
                for layer in 0..layers {
                    self.layers.push(State {
                        cache: canvas::Cache::default(),
                    });
                    self.clear_cache(layer);
                }

                self.json_tools = json_tools;
            }
        }
        Command::none()
    }
}

impl<'a> From<&'a Canvas> for Element<'a, Message, Theme, Renderer> {
    fn from(value: &'a Canvas) -> Self {
        Element::new(CanvasVessel::new(value))
            .map(|action| Message::DoAction(Box::new(DrawingAction::CanvasAction(action))))
    }
}

/// A struct that holds the [canvas](canvas::Canvas) objects for each layer, and handles the interaction.
struct CanvasVessel<'a>
{
    /// The width of the [Canvas].
    width: Length,

    /// The height of the [Canvas].
    height: Length,

    /// The list of caches for each [Layer].
    states: &'a [State],

    /// The list of [canvas layers](Canvas).
    layers: Box<Vec<canvas::Canvas<Layer<'a>, CanvasAction, Theme, Renderer>>>,

    /// The index of the currently active [Layer].
    current_layer: usize,
}

impl<'a> CanvasVessel<'a>
{
    /// Creates a new [Canvas] widget.
    fn new(canvas: &'a Canvas) -> Self {
        let mut vessel = CanvasVessel {
            width: canvas.width,
            height: canvas.height,
            states: &canvas.layers,
            layers: Box::new(vec![]),
            current_layer: canvas.current_layer,
        };

        vessel.layers = Box::new(
            vessel
                .states
                .iter()
                .enumerate()
                .map(|(pos, state)| {
                    canvas::Canvas::new(Layer {
                        state: Some(&state.cache),
                        tools: &(canvas.tool_layers[pos]),
                        current_tool: &canvas.current_tool,
                        style: &canvas.style,
                        active: pos == vessel.current_layer,
                    })
                })
                .collect(),
        );

        vessel
    }
}

impl<'a> Widget<CanvasAction, Theme, Renderer> for CanvasVessel<'a>
{
    fn size(&self) -> Size<Length> {
        Size::new(
            self.width,
            self.height
        )
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.loose().width(self.width).height(self.height);
        let mut nodes = vec![];

        for (index, layer) in (0..self.layers.len()).zip(self.layers.deref()) {
            nodes.push(layer.layout(&mut tree.children[index], renderer, &limits));
        }

        Node::with_children(
            nodes[0].size(),
            nodes
        )
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

        for (layer, index) in self.layers.iter().zip(0..self.layers.len()) {
            layer.draw(
                &state.children[index],
                renderer,
                theme,
                style,
                children.next().expect(&*format!("Canvas needs to have at least {} layers.", index)),
                cursor,
                viewport
            );
        }
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<<Layer<'_> as canvas::Program<CanvasAction, Theme, Renderer>>::State>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(<Layer<'_> as canvas::Program<
            CanvasAction,
            Theme,
            Renderer,
        >>::State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.layers.iter().map(
            |layer| Tree::new(layer as &dyn Widget<CanvasAction, Theme, Renderer>)
        ).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(
            self.layers.iter().map(
                |layer| layer as &dyn Widget<CanvasAction, Theme, Renderer>
            ).collect::<Vec<&dyn Widget<CanvasAction, Theme, Renderer>>>().as_slice()
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
        shell: &mut Shell<'_, CanvasAction>,
        viewport: &Rectangle,
    ) -> Status {
        let layer = &mut self.layers[self.current_layer];
        let mut children = layout.children();
        let binding = Node::default();
        let mut layout = Layout::new(&binding);

        for _ in 0..=self.current_layer {
            layout = children.next().expect(&*format!("Canvas needs to have at least {} children.", self.current_layer));
        }

        layer.on_event(
            &mut state.children[self.current_layer],
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
        let mut children = layout.children();
        let binding = Node::default();
        let mut layout = Layout::new(&binding);
        for _ in 0..=self.current_layer {
            layout = children.next().expect(&*format!("Canvas needs to have at least {} children.", self.current_layer));
        }
        
        self.layers[self.current_layer].mouse_interaction(state, layout, cursor, viewport, renderer)
    }
}
