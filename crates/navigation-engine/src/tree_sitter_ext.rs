use tree_sitter::Node;

pub trait NodeExt<'a> {
    fn child_at(&self, i: usize) -> Option<Node<'a>>;
    fn named_child_at(&self, i: usize) -> Option<Node<'a>>;
}

impl<'a> NodeExt<'a> for Node<'a> {
    fn child_at(&self, i: usize) -> Option<Node<'a>> {
        self.child(i as u32)
    }

    fn named_child_at(&self, i: usize) -> Option<Node<'a>> {
        self.named_child(i as u32)
    }
}
