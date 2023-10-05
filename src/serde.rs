use iced::{Color, Point, Vector};
use mongodb::bson::{Bson, doc, Document};

pub trait Serialize {
    fn serialize(&self) -> Document;
}

pub trait Deserialize {
    fn deserialize(document: Document) -> Self where Self:Sized;
}

impl Serialize for Vector {
    fn serialize(&self) -> Document {
        doc! {
            "x": self.x,
            "y": self.y,
        }
    }
}

impl Serialize for Point {
    fn serialize(&self) -> Document {
        doc! {
            "x": self.x,
            "y": self.y,
        }
    }
}

impl Serialize for Color {
    fn serialize(&self) -> Document {
        doc! {
            "r": self.r,
            "g": self.g,
            "b": self.b,
            "a": self.a,
        }
    }
}

impl Deserialize for Vector {
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

impl Deserialize for Point {
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

impl Deserialize for Color {
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