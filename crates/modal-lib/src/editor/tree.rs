use super::ManagedEditor;
use thunderdome::{Arena, Index};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EditorIndex(Index);

pub struct EditorTreeNode {
    pub editor: ManagedEditor,
    parent: Option<EditorIndex>,
    children: Vec<EditorIndex>,
}

impl EditorTreeNode {
    pub fn parent(&self) -> Option<EditorIndex> {
        self.parent
    }

    pub fn children(&self) -> &[EditorIndex] {
        &self.children
    }
}

pub struct EditorTree {
    root: Index,
    entries: Arena<EditorTreeNode>,
}

impl EditorTree {
    pub fn new(editor: ManagedEditor) -> Self {
        let root_node = EditorTreeNode {
            editor,
            parent: None,
            children: Vec::new(),
        };
        let mut entries = Arena::new();
        let root = entries.insert(root_node);

        EditorTree { root, entries }
    }

    pub fn get(&self, index: EditorIndex) -> &'_ EditorTreeNode {
        &self.entries[index.0]
    }

    pub fn get_mut(&mut self, index: EditorIndex) -> &'_ mut EditorTreeNode {
        &mut self.entries[index.0]
    }

    pub fn insert(&mut self, index: EditorIndex, editor: ManagedEditor) -> EditorIndex {
        let new_idx = EditorIndex(self.entries.insert(EditorTreeNode {
            editor,
            parent: Some(index),
            children: Vec::new(),
        }));

        self.entries[index.0].children.push(new_idx);

        new_idx
    }

    pub fn remove(&mut self, index: EditorIndex) -> Option<ManagedEditor> {
        let node = self.entries.remove(index.0);

        if let Some(node) = &node {
            let parent = node.parent().expect("Removed root editor");
            self.entries[parent.0]
                .children
                .retain(|child_idx| *child_idx != index);

            for child_id in &node.children {
                self.remove(*child_id);
            }
        }

        node.map(|node| node.editor)
    }

    pub fn iter(&self) -> impl Iterator<Item = (EditorIndex, &'_ EditorTreeNode)> {
        self.entries
            .iter()
            .map(|(index, node)| (EditorIndex(index), node))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EditorIndex, &'_ mut EditorTreeNode)> {
        self.entries
            .iter_mut()
            .map(|(index, node)| (EditorIndex(index), node))
    }

    pub fn traverse_from(
        &self,
        point: EditorIndex,
        f: &mut impl FnMut(EditorIndex, &EditorTreeNode),
    ) {
        let node = &self.entries[point.0];
        f(point, node);

        for child_id in &node.children {
            self.traverse_from(*child_id, &mut *f);
        }
    }

    pub fn root(&self) -> EditorIndex {
        EditorIndex(self.root)
    }
}
