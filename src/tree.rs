use std::collections::VecDeque;

use rstar::{AABB, PointDistance, RTree, RTreeObject};

pub trait Shape {
    fn contains_shape(&self, rhs: &Self) -> bool;
    fn contains_point(&self, point: [f32; 2]) -> bool;
    fn bounding_rect(&self) -> ([f32; 2], [f32; 2]); // [min.x, min.y, max.x, max.y]
    fn center_point(&self) -> [f32; 2];
    fn area(&self) -> f32;
}

pub type AABBType = [f32; 2];

#[derive(Debug, Clone)]
pub struct Tree<T>
where
    T: Shape + Clone,
{
    root: Option<TreeNode<T>>,
}

#[derive(Debug, Clone)]
pub struct TreeNode<T>
where
    T: Shape + Clone,
{
    value: T,
    bounding_rect: AABB<AABBType>,
    center_point: [f32; 2],
    children: RTree<TreeNode<T>>,
    area: f32,
}

// MARK: Tree

impl<T> Tree<T>
where
    T: Shape + Clone,
{
    pub fn iter(&self) -> TreeNodeDepthIterator<T> {
        if let Some(root) = &self.root {
            root.iter()
        } else {
            TreeNodeDepthIterator {
                order: Default::default(),
            }
        }
    }

    pub fn root(&self) -> &Option<TreeNode<T>> {
        &self.root
    }
}

impl<T> From<Vec<T>> for Tree<T>
where
    T: Shape + Clone + Default,
{
    fn from(value: Vec<T>) -> Self {
        Self::from((value, Default::default()))
    }
}

impl<T> From<(Vec<T>, T)> for Tree<T>
where
    T: Shape + Clone,
{
    fn from(value: (Vec<T>, T)) -> Self {
        let mut root = TreeNode {
            value: value.1,
            bounding_rect: AABB::from_corners([0.0, 0.0], [0.0, 0.0]),
            center_point: [0.0, 0.0],
            children: Default::default(),
            area: 0.0,
        };
        for x in value.0 {
            root.add_node(x);
        }

        Self { root: Some(root) }
    }
}

impl<T> FromIterator<T> for Tree<T>
where
    T: Shape + Clone + Default,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Tree<T> {
        let mut arr: Vec<TreeNode<T>> = iter.into_iter().map(|elem| elem.into()).collect();
        arr.sort_by(|l, r| r.area.partial_cmp(&l.area).unwrap());

        let mut root: TreeNode<T> = TreeNode::from(T::default());

        for shape in arr {
            root.add_node(shape);
        }

        Tree { root: Some(root) }
    }
}

// MARK: TreeNode

impl<T> RTreeObject for TreeNode<T>
where
    T: Shape + Clone,
{
    type Envelope = AABB<AABBType>;

    fn envelope(&self) -> Self::Envelope {
        self.bounding_rect.clone()
    }
}

impl<T> PointDistance for TreeNode<T>
where
    T: Shape + Clone,
{
    fn distance_2(
        &self,
        point: &<Self::Envelope as rstar::Envelope>::Point,
    ) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
        if self.value.contains_point(*point) {
            0.0
        } else {
            self.center_point.distance_2(point)
        }
    }
}

impl<T> From<T> for TreeNode<T>
where
    T: Shape + Clone,
{
    fn from(value: T) -> Self {
        let r = value.bounding_rect();
        let center_point = value.center_point();
        let area = value.area();
        Self {
            value,
            bounding_rect: AABB::from_corners(r.0, r.1),
            center_point,
            children: RTree::new(),
            area,
        }
    }
}

impl<T> TreeNode<T>
where
    T: Shape + Clone,
{
    pub fn add_node<E>(&mut self, elem: E)
    where
        E: Into<TreeNode<T>>,
    {
        let elem = elem.into();

        for child in &mut self.children {
            if child.add_node_tree_node(&elem) {
                return;
            }
        }

        self.children.insert(elem.clone());
    }

    fn add_node_tree_node(&mut self, elem: &TreeNode<T>) -> bool {
        if self.value.contains_shape(&elem.value) {
            for child in self.children.locate_all_at_point_mut(&elem.center_point) {
                if child.add_node_tree_node(elem) {
                    return true;
                }
            }

            self.children.insert(elem.clone());
            return true;
        }
        false
    }

    fn iter(&self) -> TreeNodeDepthIterator<T> {
        let mut queue = VecDeque::new();
        for child in &self.children {
            queue.push_back((0, child));
        }
        TreeNodeDepthIterator { order: queue }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn children<'a>(&'a self) -> Vec<&'a TreeNode<T>> {
        let mut children = Vec::new();

        for child in &self.children {
            children.push(child);
        }

        return children;
    }
}

// MARK: Iterator

#[derive(Debug, Clone)]
pub struct TreeNodeDepthIterator<'a, T>
where
    T: Shape + Clone,
{
    order: VecDeque<(usize, &'a TreeNode<T>)>,
}

impl<'a, T> Iterator for TreeNodeDepthIterator<'a, T>
where
    T: Shape + Clone,
{
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let Some((depth, tree_node)) = self.order.pop_front() else {
            return None;
        };

        for child in &tree_node.children {
            self.order.push_back((depth + 1, child));
        }

        Some((depth, &tree_node.value))
    }
}

#[derive(Debug, Clone)]
pub struct TreeNodeDepthIntoIterator<T>
where
    T: Shape + Clone,
{
    order: VecDeque<(usize, TreeNode<T>)>,
}

impl<T> Iterator for TreeNodeDepthIntoIterator<T>
where
    T: Shape + Clone,
{
    type Item = (usize, T);

    fn next(&mut self) -> Option<Self::Item> {
        let Some((depth, tree_node)) = self.order.pop_front() else {
            return None;
        };

        for child in tree_node.children {
            self.order.push_back((depth + 1, child));
        }

        Some((depth, tree_node.value))
    }
}

impl<T> IntoIterator for Tree<T>
where
    T: Shape + Clone,
{
    type Item = (usize, T);

    type IntoIter = TreeNodeDepthIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        if let Some(root) = self.root {
            let mut queue = VecDeque::new();
            for child in root.children {
                queue.push_back((0, child));
            }
            TreeNodeDepthIntoIterator { order: queue }
        } else {
            TreeNodeDepthIntoIterator {
                order: Default::default(),
            }
        }
    }
}

#[cfg(feature = "geo-integration")]
mod geo_impls {
    use crate::*;
    use geo::{Area, Contains, InteriorPoint, Polygon};

    impl Shape for Polygon {
        fn contains_shape(&self, rhs: &Self) -> bool {
            if self.unsigned_area() < rhs.unsigned_area() {
                return false;
            }

            if let Some(point) = rhs.interior_point() {
                self.contains(&point)
            } else {
                false
            }
        }

        fn contains_point(&self, point: [f32; 2]) -> bool {
            self.contains(&geo::coord! {
                x: point[0] as f64,
                y: point[1] as f64,
            })
        }

        fn bounding_rect(&self) -> ([f32; 2], [f32; 2]) {
            if let Some(rect) = geo::algorithm::bounding_rect::BoundingRect::bounding_rect(self) {
                let ((x1, y1), (x2, y2)) = (rect.min().x_y(), rect.max().x_y());
                ([x1 as f32, y1 as f32], [x2 as f32, y2 as f32])
            } else {
                panic!("Could not get bounding rect.");
            }
        }

        fn center_point(&self) -> [f32; 2] {
            if let Some(point) = self.interior_point() {
                let (x, y) = point.x_y();
                [x as f32, y as f32]
            } else {
                panic!("Could not get center point.");
            }
        }

        fn area(&self) -> f32 {
            self.unsigned_area() as f32
        }
    }

    impl Tree<Polygon> {
        pub fn from_polygon(mut value: Vec<Polygon>) -> Self {
            value.sort_by(|l, r| r.unsigned_area().partial_cmp(&l.unsigned_area()).unwrap());
            Self::from((
                value,
                Polygon::new(geo::LineString::new(Vec::new()), Vec::new()),
            ))
        }
    }

    impl<T> Shape for (T, Polygon) {
        fn contains_shape(&self, rhs: &Self) -> bool {
            self.1.contains_shape(&rhs.1)
        }

        fn contains_point(&self, point: [f32; 2]) -> bool {
            self.1.contains_point(point)
        }

        fn bounding_rect(&self) -> ([f32; 2], [f32; 2]) {
            self.1.bounding_rect()
        }

        fn center_point(&self) -> [f32; 2] {
            self.1.center_point()
        }

        fn area(&self) -> f32 {
            self.1.area()
        }
    }

    impl<T: Clone + Default> Tree<(T, Polygon)> {
        pub fn from_polygon_id(mut value: Vec<(T, Polygon)>) -> Self {
            value.sort_by(|l, r| {
                r.1.unsigned_area()
                    .partial_cmp(&l.1.unsigned_area())
                    .unwrap()
            });
            Self::from((
                value,
                (
                    T::default(),
                    Polygon::new(geo::LineString::new(Vec::new()), Vec::new()),
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use geo::Polygon;

    use crate::*;

    #[test]
    fn it_works() {
        let path = Path::new("/home/cameron/Downloads/CAM.svg");
        let lines = import_svg(path, 0.0001).unwrap();

        let polygons: Vec<Polygon> = lines
            .into_iter()
            .map(|line| Polygon::new(line, Vec::new()))
            .collect();

        let tree: Tree<Polygon> = Tree::from_polygon(polygons);
        let mut _count = 0;
        for (_depth, _) in tree.iter() {
            // println!("Depth: {}", depth);
            _count += 1;
        }

        // println!("COUNT: {}", count);
    }
}
