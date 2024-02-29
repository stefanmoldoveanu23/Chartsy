use std::collections::BTreeMap;
use std::path::Path;
use svg::node::element::{Group, Rectangle};
use svg::Document;

/// Data for a svg object.
/// The [BTreeMap] is used to get the latest added tool for undo-ing.
#[derive(Debug, Clone)]
pub struct SVG {
    tools: Vec<Vec<(Group, usize)>>,
    group_order: BTreeMap<usize, usize>,
    tool_count: usize,
}

impl SVG {
    /// Create a new svg with the given amount of layers.
    pub fn new(cnt_layers: usize) -> Self {
        Self {
            tools: vec![vec![]; cnt_layers],
            group_order: BTreeMap::new(),
            tool_count: 0,
        }
    }

    /// Add a new layer.
    pub fn add_layer(&mut self) {
        self.tools.push(vec![]);
    }

    /// Add a new tool serialized as a [Group] to the given layer.
    pub fn add_tool(&mut self, layer: usize, data: Group) {

        let last_order = self.tools[layer].last();
        if let Some(last_order) = last_order {
            self.group_order.remove(&last_order.1);
        }

        self.group_order.insert(self.tool_count, layer);

        self.tools[layer].push((data, self.tool_count));
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
        self.tools[layer].pop();

        if self.tools[layer].len() > 0 {
            self.group_order
                .insert(self.tools[layer].last().unwrap().1, layer);
        }

        self.tool_count -= 1;
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

        for layer in &self.tools {
            let mut group = Group::new();

            for (tool, _) in layer {
                group = group.add(tool.clone());
            }
            tools = tools.add(group);
        }

        Document::new()
            .set("viewBox", (0.0, 0.0, 800.0, 600.0))
            .add(background)
            .add(tools)
    }
}
