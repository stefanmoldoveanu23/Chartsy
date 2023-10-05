use std::f32::consts::PI;
use std::fmt::{Debug};
use std::ops::{Add, Sub};
use iced::{Color, Point, Vector};
use iced::widget::canvas::{Fill, Frame, Path};
use crate::canvas::tool::Tool;
use rand::{rngs::StdRng, SeedableRng, Rng};
use crate::canvas::style::Style;

use crate::canvas::tools::brush::Brush;

#[derive(Default, Debug, Clone)]
struct Seed(pub [u8; 32]);

impl Seed {
    fn new(point1: Point, point2: Point) -> Self {
        let mut seed = [0; 32];
        let vec = point2.sub(point1);

        for i in 0..16 {
            seed[2 * i] = ((point1.x.clone() as u32 % 128) as u8) + (i.clone() as u8 * ((vec.x.clone() / 16.0) as u32 % 128) as u8);
            seed[2 * i.clone() + 1] = ((point1.y.clone() as u32 % 128) as u8) + (i.clone() as u8 * ((vec.y.clone() / 16.0) as u32 % 128) as u8);
        }

        Seed(seed)
    }
}

impl AsMut<[u8]> for Seed {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
struct RNG(Seed);

impl SeedableRng for RNG {
    type Seed = Seed;

    fn from_seed(seed: Self::Seed) -> Self {
        Self(seed)
    }
}


#[derive(Debug, Clone)]
pub struct Airbrush {
    start: Point,
    offsets: Vec<Vector>,
    style: Style,
}

impl Airbrush {
    fn spray(point: Point, rng: &mut StdRng, frame: &mut Frame, style: Style) {
        let mut radius = 1.2;
        let mut fill = Color::BLACK;
        if let Some((width, color, _, _)) = style.stroke {
            radius = width;
            fill = color;
        }

        let spray = Path::new(|builder| {
            for _ in 0..5 {
                let offset = Vector::new(10.0 * (rng.gen_range(0.0..1.0) * 2.0 * PI).cos(), 10.0 * (rng.gen_range(0.0..1.0) * 2.0 * PI).sin());

                builder.circle(point.add(offset), radius);
            }
        });

        frame.fill(&spray, Fill::from(fill));
    }
}


impl Brush for Airbrush {
    fn new(start: Point, offsets: Vec<Vector>, style: Style) -> Self where Self: Sized {
        Airbrush { start, offsets, style }
    }

    fn id() -> String where Self: Sized {
        String::from("Airbrush")
    }

    fn get_start(&self) -> Point {
        self.start
    }

    fn get_offsets(&self) -> Vec<Vector> {
        self.offsets.clone()
    }
    fn get_style(&self) -> Style {
        self.style.clone()
    }

    fn add_stroke_piece(point1: Point, point2: Point, frame: &mut Frame, style: Style) where Self: Sized {
        let rng = RNG(Seed::new(point1, point2));
        let mut rng = StdRng::from_seed(rng.0.0);

        Airbrush::spray(point1, &mut rng, frame, style);
    }

    fn add_end(point: Point, frame: &mut Frame, style: Style) where Self: Sized {
        let rng = RNG(Seed::new(point, Point::new(0.0, 0.0)));
        let mut rng = StdRng::from_seed(rng.0.0);

        Airbrush::spray(point, &mut rng, frame, style);
    }
}

impl Into<Box<dyn Tool>> for Box<Airbrush> {
    fn into(self) -> Box<dyn Tool> {
        self.boxed_clone()
    }
}