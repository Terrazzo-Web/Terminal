#![cfg(feature = "client")]

use std::sync::Arc;

use terrazzo::html;
use terrazzo::prelude::*;

use super::file_path::FilePath;
use super::manager::TextEditorManager;

mod graph;
mod graphx;

stylance::import_style!(style, "graph.scss");

const SVG_XMLNS: &str = "http://www.w3.org/2000/svg";

#[html(html_tags = [div, svg, circle])]
pub fn graph_editor(
    _manager: Ptr<TextEditorManager>,
    _path: FilePath<Arc<str>>,
    _content: Arc<str>,
) -> XElement {
    div(
        style::width = "100%",
        style::height = "100%",
        class = style::graph,
        svg(
            xmlns = SVG_XMLNS,
            html::circle(
                xmlns = SVG_XMLNS,
                cx = "100",
                cy = "100",
                r = "80",
                fill = "lightblue",
                stroke = "green",
                stroke_width = "3",
            ),
        ),
    )
}
