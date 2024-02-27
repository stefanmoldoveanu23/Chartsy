use std::collections::BTreeMap;
use std::path::Path;
use svg::Document;
use svg::node::element::{Group, Rectangle};

#[derive(Debug, Clone)]
pub struct SVG
{
    groups: Vec<Group>,
    tool_order: Vec<Vec<usize>>,
    group_order: BTreeMap<usize, usize>,
    tool_count: usize,
}

impl SVG
{
    pub fn new(cnt_layers: usize) -> Self
    {
        Self {
            groups: vec![Group::new(); cnt_layers],
            tool_order: vec![vec![]; cnt_layers],
            group_order: BTreeMap::new(),
            tool_count: 0,
        }
    }

    pub fn add_layer(&mut self)
    {
        self.groups.push(Group::new());
        self.tool_order.push(vec![]);
    }

    pub fn add_tool(&mut self, layer: usize, data: Group)
    {
        self.groups[layer] = self.groups[layer].clone().add(data);

        let last_order = self.tool_order[layer].last();
        if let Some(last_order) = last_order {
            self.group_order.remove(last_order);
        }

        self.group_order.insert(self.tool_count, layer);

        self.tool_order[layer].push(self.tool_count);
        self.tool_count += 1;
    }

    pub fn get_cnt_layers(&self) -> usize
    {
        self.groups.len()
    }

    pub fn remove(&mut self)
    {
        let last = self.group_order.pop_last();
        if last.is_none() {
            return;
        }

        let last = last.unwrap();
        self.tool_order[last.1].pop();

        if self.tool_order[last.1].len() > 0 {
            self.group_order.insert(*self.tool_order[last.1].last().unwrap(), last.1);
        }

        self.groups[last.1].get_children_mut().pop();

        self.tool_count -= 1;
    }

    pub fn save<T>(self, path: T)
    where T: AsRef<Path>
    {
        let document :Document= self.as_document();
        if let Err(e) = svg::save(path, &document) {
            println!("Error saving svg document: {}", e);
        }
    }

    pub fn as_document(&self) -> Document
    {
        let background = Rectangle::new()
            .set("x", 0.0)
            .set("y", 0.0)
            .set("width", 800.0)
            .set("height", 600.0)
            .set("fill", "white");

        let mut tools = Group::new()
            .set("style", "isolation:isolate");

        for group in &self.groups {
            tools = tools.add(group.clone());
        }

        Document::new()
            .set("viewBox", (0.0, 0.0, 800.0, 600.0))
            .add(background)
            .add(tools)
    }
}