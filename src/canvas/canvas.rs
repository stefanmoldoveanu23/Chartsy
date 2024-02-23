use std::sync::Arc;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::advanced::widget::{Tree, tree};
use iced::{Element, Event, Length, Rectangle, Size, Renderer, Command};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::canvas;
use mongodb::bson::{doc, Document, Uuid};
use crate::canvas::layer::{CanvasAction, Layer};
use crate::canvas::style::Style;
use crate::mongo::{MongoRequest, MongoRequestType, MongoResponse};
use crate::scene::Message;
use crate::scenes::drawing::DrawingAction;
use crate::theme::Theme;
use super::tool::{self, Pending, Tool};
use super::tools::line::LinePending;

pub struct State {
    cache: canvas::Cache,
}

pub struct Canvas {
    pub(crate) id: Uuid,
    pub(crate) width: Length,
    pub(crate) height: Length,
    pub(crate) layers: Box<Vec<State>>,
    pub(crate) current_layer: usize,
    pub(crate) tools: Box<Vec<(Arc<dyn Tool>, usize)>>,
    pub(crate) undo_stack: Box<Vec<(Arc<dyn Tool>, usize)>>,
    pub(crate) tool_layers: Box<Vec<Vec<Arc<dyn Tool>>>>,
    pub(crate) count_saved: usize,
    pub(crate) default_tool: Box<dyn Pending>,
    pub(crate) current_tool: Box<dyn Pending>,
    pub(crate) style: Style,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl Canvas {
    pub(crate) fn new() -> Self {
        Canvas {
            id: Uuid::new(),
            width: Length::Fill,
            height: Length::Fill,
            layers: Box::new(vec![State {cache: canvas::Cache::default()}]),
            current_layer: 0,
            tools: Box::new(vec![]),
            undo_stack: Box::new(vec![]),
            tool_layers: Box::new(vec![vec![]]),
            count_saved: 0,
            default_tool: Box::new(LinePending::None),
            current_tool: Box::new(LinePending::None),
            style: Style::default(),
        }
    }

    pub fn get_layer_count(&self) -> usize {
        self.layers.len()
    }

    fn get_tools_serialized(&self) -> Vec<Document> {
        let mut vec = vec![];

        for pos in self.count_saved..self.tools.len() {
            let val = self.tools.get(pos);

            if let Some((tool, layer)) = val {
                let mut document = tool.serialize();
                document.insert("order", pos.clone() as u32);
                document.insert("canvas_id", self.id);
                document.insert("name", tool.id());
                document.insert("layer", *layer as u32);

                vec.push(document);
            }
        }

        vec
    }

    fn clear_cache(&mut self, layer: usize) {
        let layer = self.layers.get(layer);
        if let Some(state) = layer {
            state.cache.clear();
        }
    }

    pub(crate) fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub(crate) fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub(crate) fn update(&mut self, message: CanvasAction) -> Command<Message> {

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
                self.layers.push(State {cache: canvas::Cache::default()});
                self.current_tool = self.default_tool.clone();
                self.current_layer = self.layers.len() - 1;

                let id = self.id.clone();
                let layers = self.layers.len();
                return Command::perform(
                    async {},
                    move |_| {
                        Message::SendMongoRequests(
                            vec![
                                MongoRequest::new(
                                    "canvases".into(),
                                    MongoRequestType::Update(
                                        doc! { "id": id },
                                        doc! { "$set": { "layers": layers as u32 } }
                                    )
                                )
                            ],
                            |_| {
                                Box::new(DrawingAction::None)
                            }
                        )
                    }
                );
            }
            CanvasAction::ActivateLayer(layer) => {
                self.current_tool = self.default_tool.clone();
                self.current_layer = layer;
            }
            CanvasAction::Save => {
                let tools = self.get_tools_serialized();
                if tools.is_empty() {
                    return Command::none();
                }
                let delete_lower_bound = self.count_saved;
                let delete_upper_bound = self.tools.len();

                return
                    Command::perform(
                        async {},
                        move |_| {
                            Message::SendMongoRequests(
                                vec![
                                    MongoRequest::new(
                                        "tools".into(),
                                        MongoRequestType::Delete(
                                            doc!{"order": {
                                                "$gte": delete_lower_bound as u32,
                                                "$lte": delete_upper_bound as u32,
                                            }}
                                        )
                                    ),
                                    MongoRequest::new(
                                        "tools".into(),
                                        MongoRequestType::Insert(tools),
                                    )
                                ],
                                move |responses| {
                                    for response in responses {
                                        if let MongoResponse::Insert(result) = response {
                                            return Box::new(DrawingAction::CanvasAction(CanvasAction::Saved(Arc::new(result))));
                                        }
                                        break
                                    }

                                    Box::new(DrawingAction::None)
                                }
                            )
                        }
                    );
            }
            CanvasAction::Undo => {
                if self.tools.len() > 0 {
                    let opt = self.tools.pop();
                    if let Some((tool, layer)) = opt {
                        self.tool_layers[layer].pop();
                        self.undo_stack.push((tool.clone(), layer));

                        self.clear_cache(layer);
                    }

                    if self.count_saved > self.tools.len() {
                        self.count_saved -= 1;
                    }
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
            CanvasAction::Saved(insert_result) => {
                self.count_saved += insert_result.inserted_ids.len();
            }
            CanvasAction::Loaded(layers, tools) => {
                self.tools = Box::new(vec![]);
                self.tool_layers = Box::new(vec![vec![]; layers]);
                self.layers = Box::new(vec![]);

                for tool in tools {
                    let tool = tool::get_deserialized(tool.clone());

                    match tool {
                        Some((tool, layer)) => {
                            self.tools.push((tool.clone(), layer));
                            self.tool_layers[layer].push(tool.clone());
                        }
                        None => {
                            eprintln!("Failed to get correct type of tool.");
                        }
                    }
                }

                self.count_saved = self.tools.len();
                for layer in 0..layers {
                    self.layers.push(State {cache: canvas::Cache::default()});
                    self.clear_cache(layer);
                }
            }
        }
        Command::none()
    }
}

impl<'a> From<&'a Canvas> for Element<'a, Message, Renderer<Theme>> {
    fn from(value: &'a Canvas) -> Self {
        Element::new(CanvasVessel::new(value)).map(|action| {Message::DoAction(Box::new(DrawingAction::CanvasAction(action)))})
    }
}

struct CanvasVessel<'a> {
    width: Length,
    height: Length,
    states: &'a [State],
    layers: Box<Vec<canvas::Canvas<Layer<'a>, CanvasAction, Renderer<Theme>>>>,
    current_layer: usize,
}

impl<'a> CanvasVessel<'a> {
    fn new(canvas: &'a Canvas) -> Self {
        let mut vessel = CanvasVessel {
            width: canvas.width,
            height: canvas.height,
            states: &canvas.layers,
            layers: Box::new(vec![]),
            current_layer: canvas.current_layer,
        };

        vessel.layers = Box::new(
            vessel.states.iter().enumerate().map(|(pos, state)| {
                canvas::Canvas::new(
                    Layer {
                        state: Some(&state.cache),
                        tools: &(canvas.tool_layers[pos]),
                        current_tool: &canvas.current_tool,
                        style: &canvas.style,
                        active: pos == vessel.current_layer,
                    }
                )
            }).collect()
        );

        vessel

    }
}

impl<'a> Widget<CanvasAction, Renderer<Theme>> for CanvasVessel<'a> {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(
        &self,
        _renderer: &Renderer<Theme>,
        limits: &Limits
    ) -> Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.resolve(Size::ZERO);

        Node::new(size)
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer<Theme>,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle
    ) {
        for layer in self.layers.iter() {
            layer.draw(
                state,
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        }
    }

    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<<Layer<'_> as canvas::Program<CanvasAction, Renderer<Theme>>>::State>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(<Layer<'_> as canvas::Program<CanvasAction, Renderer<Theme>>>::State::default())
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer<Theme>,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, CanvasAction>,
        viewport: &Rectangle
    ) -> Status {
        let layer = &mut self.layers[self.current_layer];
        layer.on_event(
            state,
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
        renderer: &Renderer<Theme>
    ) -> Interaction {
        self.layers[self.current_layer].mouse_interaction(
            state,
            layout,
            cursor,
            viewport,
            renderer
        )
    }
}