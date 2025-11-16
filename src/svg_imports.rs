#[cfg(feature = "svg-integration")]
use geo::LineString;

#[cfg(feature = "svg-integration")]
pub fn import_svg(path: &std::path::Path, flatten: f32) -> Option<Vec<LineString>> {
    let Ok(content) = std::fs::read_to_string(path).map_err(|e| e.to_string()) else {
        return None;
    };
    Some(import_to_lines(&content, flatten))
}

#[cfg(feature = "svg-integration")]
pub fn import_to_lines(svg: &str, flatten: f32) -> Vec<LineString> {
    use geo::coord;

    let tree = usvg::Tree::from_str(svg, &usvg_options().to_ref()).expect("Could not read svg");

    let svg = tree.svg_node();
    let vb = svg.view_box.rect;

    let width_in_inches = svg.size.width() / 96.0;
    let height_in_inches = svg.size.height() / 96.0;

    let scale_x = width_in_inches / vb.width();
    let scale_y = height_in_inches / vb.height();

    let root = tree.root();
    let mut line_strings: Vec<LineString> = Vec::new();
    let mut points = Vec::new();
    for n in root.descendants() {
        if let usvg::NodeKind::Path(ref p) = *n.borrow() {
            let path = lyon_path_from_data(&p.data);

            use lyon::path::iterator::PathIterator;
            let flattened_iter = path.iter().flattened(flatten);
            for evt in flattened_iter {
                match evt {
                    lyon::path::PathEvent::Begin { at } => {
                        points.push(coord! { x: at.x as f64 * scale_x, y: -at.y as f64 * scale_y});
                    }
                    lyon::path::PathEvent::Line { from: _, to } => {
                        points.push(coord! { x: to.x as f64 * scale_x, y: -to.y as f64 * scale_y});
                    }
                    lyon::path::PathEvent::End {
                        last: _,
                        first,
                        close: _,
                    } => {
                        points.push(
                            coord! { x: first.x as f64 * scale_x, y: -first.y as f64 * scale_y},
                        );
                        line_strings.push(LineString::new(points.clone()));
                        points.clear();
                    }
                    _ => {
                        panic!()
                    }
                }
            }
        }
    }

    line_strings
}

#[cfg(feature = "svg-integration")]
fn lyon_path_from_data(data: &usvg::PathData) -> lyon::path::Path {
    use lyon::geom::euclid;

    let mut path = lyon::path::Path::svg_builder();
    for cmd in data.0.iter() {
        match cmd {
            usvg::PathSegment::MoveTo { x, y } => {
                path.move_to(euclid::point2(*x as f32, *y as f32));
            }
            usvg::PathSegment::LineTo { x, y } => {
                path.line_to(euclid::point2(*x as f32, *y as f32));
            }
            usvg::PathSegment::ClosePath => {
                path.close();
            }
            usvg::PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let ctrl1 = euclid::point2(*x1 as f32, *y1 as f32);
                let ctrl2 = euclid::point2(*x2 as f32, *y2 as f32);
                let to = euclid::point2(*x as f32, *y as f32);
                path.cubic_bezier_to(ctrl1, ctrl2, to);
            }
        }
    }

    path.build()
}

#[cfg(feature = "svg-integration")]
pub fn usvg_options() -> usvg::Options {
    let mut options = usvg::Options::default();
    // PERF: This will cause us to load the system font every time we load a svg. This could
    // portentially be a performance problem or it may be an easy way to allow new fonts to be
    // loaded in as they're added to the system.
    options.fontdb.load_system_fonts();

    // Having a font that is always loaded allows our tests to use this font without having to
    // worry about it being installed on every developers machine.
    // let test_font: &[u8] = include_bytes!("../../../test_data/fonts/OpenSans-Regular.ttf");
    // let test_font: Vec<_> = Vec::from(test_font);

    // options.fontdb.load_font_data(test_font);
    options
}
