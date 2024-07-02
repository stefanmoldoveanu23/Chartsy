use mongodb::bson::Uuid;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use svg::node::element::{Group, Rectangle};
use svg::Document;

/// Data for a svg object.
#[derive(Debug, Clone)]
pub struct SVG {
    /// The list of tools organized by layer.
    tools: HashMap<Uuid, Vec<(Group, usize)>>,

    /// Used to get the latest added tool for undo-ing.
    group_order: BTreeMap<usize, Uuid>,

    /// The total amount of tools.
    tool_count: usize,

    /// The order of the layers.
    layer_order: Vec<Uuid>,
}

impl SVG {
    /// Create a new svg with the given amount of layers.
    pub fn new(layers: &Vec<Uuid>) -> Self {
        Self {
            tools: HashMap::from_iter(layers.iter().map(|id| (*id, vec![]))),
            group_order: BTreeMap::new(),
            tool_count: 0,
            layer_order: layers.clone(),
        }
    }

    pub fn add_layer(&mut self, layer_id: Uuid) {
        self.tools.insert(layer_id, vec![]);
        self.layer_order.push(layer_id);
    }

    /// Add a new tool serialized as a [Group] to the given layer.
    pub fn add_tool(&mut self, layer: &Uuid, data: Group) {
        let last_order = self.tools[layer].last();
        if let Some(last_order) = last_order {
            self.group_order.remove(&last_order.1);
        }

        self.group_order.insert(self.tool_count, *layer);

        self.tools
            .get_mut(layer)
            .unwrap()
            .push((data, self.tool_count));
        self.tool_count += 1;
    }

    /// Returns the number of layers.
    pub fn get_cnt_layers(&self) -> usize {
        self.tools.len()
    }

    /// Returns the amount of tools.
    pub fn get_tool_count(&self) -> usize {
        self.tool_count
    }

    /// Remove the latest added tool using the [BTreeMap].
    pub fn remove(&mut self) {
        let last = self.group_order.pop_last();
        if last.is_none() {
            return;
        }

        let (_, layer) = last.unwrap();
        self.tools.get_mut(&layer).unwrap().pop();

        if self.tools[&layer].len() > 0 {
            self.group_order
                .insert(self.tools[&layer].last().unwrap().1, layer);
        }

        self.tool_count -= 1;
    }

    /// Removes the given layer.
    pub fn remove_layer(&mut self, layer_id: &Uuid) {
        self.group_order.retain(|_, layer| *layer != *layer_id);
        self.tool_count -= self.tools.get(layer_id).unwrap().len();
        self.tools.remove(layer_id);
        self.layer_order.retain(|id| *id != *layer_id);
    }

    /// Save the svg locally at the given path;
    pub fn save<T>(self, path: T)
    where
        T: AsRef<Path>,
    {
        let document: Document = self.as_document();
        if let Err(e) = svg::save(path, &document) {
            println!("Error saving svg document: {}", e);
        }
    }

    /// Convert the [SVG] to a [svg document](Document).
    pub fn as_document(&self) -> Document {
        let background = Rectangle::new()
            .set("x", 0.0)
            .set("y", 0.0)
            .set("width", 800.0)
            .set("height", 600.0)
            .set("fill", "white");

        let mut tools = Group::new().set("style", "isolation:isolate");

        for layer in &self.layer_order {
            let mut group = Group::new();

            for tool in self.tools.get(layer).unwrap() {
                group = group.add(tool.0.clone());
            }
            tools = tools.add(group);
        }

        Document::new()
            .set("viewBox", (0.0, 0.0, 800.0, 600.0))
            .add(background)
            .add(tools)
    }
}
