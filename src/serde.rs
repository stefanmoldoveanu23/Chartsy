use iced::{Point, Vector};
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

impl Deserialize for Vector {
    fn deserialize(document: Document) -> Self {
        let mut vector = Vector::new(0.0, 0.0);

        if let Some(Bson::Double(x)) = document.get("x") {
                vector.x = x.clone() as f32;
        }

        if let Some(Bson::Double(y)) = document.get("y") {
                vector.y = y.clone() as f32;
        }

        vector
    }
}

impl Deserialize for Point {
    fn deserialize(document: Document) -> Self {
        let mut point = Point::new(0.0, 0.0);

        if let Some(Bson::Double(x)) = document.get("x") {
                point.x = x.clone() as f32;
        }

        if let Some(Bson::Double(y)) = document.get("y") {
                point.y = y.clone() as f32;
        }

        point
    }
}