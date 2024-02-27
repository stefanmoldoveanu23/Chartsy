use iced::{Color, Point, Vector};
use json::JsonValue;
use json::object::Object;
use mongodb::bson::{Bson, doc, Document};

/// An object with this trait can be turned into document.
pub trait Serialize<T>
where T: Clone {
    /// Serializes the object.
    fn serialize(&self) -> T;
}

/// An object with this trait can be deserialized from a document.
pub trait Deserialize<T>
where T: Clone {
    /// Deserialized the document, returning an instance.
    fn deserialize(document: T) -> Self where Self:Sized;
}

impl Serialize<Document> for Vector {
    fn serialize(&self) -> Document {
        doc! {
            "x": self.x,
            "y": self.y,
        }
    }
}

impl Serialize<Object> for Vector
{
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("x", JsonValue::Number(self.x.into()));
        data.insert("y", JsonValue::Number(self.y.into()));

        data
    }
}

impl Serialize<Document> for Point {
    fn serialize(&self) -> Document {
        doc! {
            "x": self.x,
            "y": self.y,
        }
    }
}

impl Serialize<Object> for Point
{
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("x", JsonValue::Number(self.x.into()));
        data.insert("y", JsonValue::Number(self.y.into()));

        data
    }
}

impl Serialize<Document> for Color {
    fn serialize(&self) -> Document {
        doc! {
            "r": self.r,
            "g": self.g,
            "b": self.b,
            "a": self.a,
        }
    }
}

impl Serialize<Object> for Color
{
    fn serialize(&self) -> Object {
        let mut data = Object::new();

        data.insert("r", JsonValue::Number(self.r.into()));
        data.insert("g", JsonValue::Number(self.g.into()));
        data.insert("b", JsonValue::Number(self.b.into()));
        data.insert("a", JsonValue::Number(self.a.into()));

        data
    }
}

impl Deserialize<Document> for Vector {
    fn deserialize(document: Document) -> Self {
        let mut vector = Vector::new(0.0, 0.0);

        if let Some(Bson::Double(x)) = document.get("x") {
                vector.x = *x as f32;
        }

        if let Some(Bson::Double(y)) = document.get("y") {
                vector.y = *y as f32;
        }

        vector
    }
}

impl Deserialize<Object> for Vector
{
    fn deserialize(document: Object) -> Self where Self: Sized {
        let mut vector = Vector::default();

        if let Some(JsonValue::Number(x)) = document.get("x") {
            vector.x = f32::from(*x);
        }

        if let Some(JsonValue::Number(y)) = document.get("y") {
            vector.y = f32::from(*y);
        }

        vector
    }
}

impl Deserialize<Document> for Point {
    fn deserialize(document: Document) -> Self {
        let mut point = Point::new(0.0, 0.0);

        if let Some(Bson::Double(x)) = document.get("x") {
                point.x = *x as f32;
        }

        if let Some(Bson::Double(y)) = document.get("y") {
                point.y = *y as f32;
        }

        point
    }
}

impl Deserialize<Object> for Point
{
    fn deserialize(document: Object) -> Self where Self: Sized {
        let mut point = Point::default();

        if let Some(JsonValue::Number(x)) = document.get("x") {
            point.x = f32::from(*x);
        }
        if let Some(JsonValue::Number(y)) = document.get("y") {
            point.y = f32::from(*y);
        }

        point
    }
}

impl Deserialize<Document> for Color {
    fn deserialize(document: Document) -> Self where Self: Sized {
        let mut color = Color::new(0.0, 0.0, 0.0, 1.0);

        if let Some(Bson::Double(r)) = document.get("r") {
            color.r = *r as f32
        }

        if let Some(Bson::Double(g)) = document.get("g") {
            color.g = *g as f32
        }

        if let Some(Bson::Double(b)) = document.get("b") {
            color.b = *b as f32
        }

        if let Some(Bson::Double(a)) = document.get("a") {
            color.a = *a as f32
        }

        color
    }
}

impl Deserialize<Object> for Color
{
    fn deserialize(document: Object) -> Self where Self: Sized {
        let mut color = Color::default();

        if let Some(JsonValue::Number(r)) = document.get("r") {
            color.r = f32::from(*r);
        }
        if let Some(JsonValue::Number(g)) = document.get("g") {
            color.g = f32::from(*g);
        }
        if let Some(JsonValue::Number(b)) = document.get("b") {
            color.b = f32::from(*b);
        }
        if let Some(JsonValue::Number(a)) = document.get("a") {
            color.a = f32::from(*a);
        }

        color
    }
}