use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: HashMap<i32, Arc<Node>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    pub id: i32,
    pub position: Coord,
    pub size: Coord,
    pub def: NodeDef,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeDef {
    Cicle(Circle),
    Rectangle(Rectangle),
    Line(Line),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Circle;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rectangle;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Line;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

impl Eq for Coord {}
