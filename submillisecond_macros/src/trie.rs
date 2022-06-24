use std::{fmt::Debug, mem::swap};

#[derive(Debug)]
pub struct Trie<T> {
    node: TrieNode<T>,
}

impl<T: Clone + Debug> Default for Trie<T> {
    fn default() -> Self {
        Trie {
            node: TrieNode::Empty,
        }
    }
}

impl<T: Clone + Debug> Trie<T> {
    pub fn children(&mut self) -> Children<T> {
        Children::new(TrieNode::Node {
            value: None,
            prefix: vec![],
            children: vec![Box::new(self.node.clone())],
        })
    }

    pub fn insert(&mut self, key: String, new_value: T) {
        self.node.insert(key, new_value)
    }
}

#[derive(Debug, Clone)]
pub enum TrieNode<V> {
    Node {
        value: Option<V>,
        prefix: Vec<u8>,
        children: Vec<Box<TrieNode<V>>>,
    },
    Empty,
}

impl<T: Debug + Clone> TrieNode<T> {
    pub fn create_terminal(prefix: Vec<u8>, value: T) -> Self {
        TrieNode::Node {
            value: Some(value),
            prefix,
            children: vec![],
        }
    }

    pub fn insert(&mut self, key: String, new_value: T) {
        let key_vec = key.as_bytes();
        let key_len = key.len();
        match self {
            TrieNode::Node {
                ref mut prefix,
                ref mut children,
                ref mut value,
            } => {
                let mut last_match = key_len;
                for (idx, b) in key.as_bytes().iter().enumerate() {
                    // if node prefix ended, try to delegate to a child
                    if idx >= prefix.len() {
                        return Self::delegate_to_child(
                            key[idx..].to_string(),
                            new_value,
                            children,
                        );
                    }
                    // if matches current node, delegate to a child
                    if *b == prefix[idx] {
                        continue;
                    } else {
                        // they are not the same, need to split at longest common prefix
                        last_match = idx;
                        break;
                    }
                }
                // inserting the same key
                if key_len == prefix.len() {
                    return;
                }
                // in this case, key_len will ALWAYS be shorter than prefix.len()
                let prefix_clone = prefix.clone();
                // prefix has left-over data, need to split the prefix
                let (new_prefix, suffix) = prefix_clone.split_at(last_match);
                // create new node that carries data from current node
                let mut new_children: Vec<Box<TrieNode<T>>> = vec![];
                swap(&mut new_children, children);
                let value = if value.is_some() { value.clone() } else { None };
                let new_child: Box<TrieNode<T>> = Box::new(TrieNode::Node {
                    value: value,
                    prefix: suffix.to_vec(),
                    children: vec![],
                });
                // insert new node with new suffix if
                if last_match < key_len {
                    // create new self
                    *self = TrieNode::Node {
                        value: None,
                        prefix: new_prefix.to_vec(),
                        children: vec![
                            new_child,
                            Box::new(TrieNode::Node {
                                value: Some(new_value),
                                prefix: key_vec[last_match..].to_vec(),
                                children: vec![],
                            }),
                        ],
                    };
                } else {
                    // create new self
                    *self = TrieNode::Node {
                        value: Some(new_value),
                        prefix: new_prefix.to_vec(),
                        children: vec![new_child],
                    };
                }
            }
            TrieNode::Empty => {
                let key_vec = key.as_bytes().to_vec();
                *self = TrieNode::create_terminal(key_vec, new_value);
            }
        }
    }

    fn delegate_to_child(key: String, new_value: T, children: &mut Vec<Box<TrieNode<T>>>) {
        let next = key.as_bytes()[0];
        // if we find any existing match for the next one we pass it on
        if let Some(child) = children.iter_mut().find(|c| {
            if let TrieNode::Node { prefix, .. } = c.as_ref() {
                // if first character matches we found the right child
                return prefix[0] == next;
            }
            false
        }) {
            child.as_mut().insert(key, new_value);
            return;
        } else if key.len() > 1 {
            // create a new terminal child
            children.push(Box::new(TrieNode::create_terminal(
                key.as_bytes().to_vec(),
                new_value,
            )));
        }
    }

    pub fn children(&mut self) -> Children<T> {
        Children::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Node<T> {
    pub prefix: Vec<u8>,
    pub value: Option<T>,
    pub trie_node: TrieNode<T>,
}

impl<T: Clone + Debug> Node<T> {
    pub fn children(&mut self) -> Children<T> {
        self.trie_node.children()
    }

    pub fn is_leaf(&mut self) -> bool {
        if let TrieNode::Node { ref children, .. } = self.trie_node {
            return children.is_empty();
        }
        true
    }
}

#[derive(Debug)]
pub struct Children<T>
where
    T: Debug + Clone,
{
    trie: TrieNode<T>,
    idx_child: usize,
}

impl<T: Debug + Clone> Children<T> {
    pub fn new(trie: TrieNode<T>) -> Self {
        Children { trie, idx_child: 0 }
    }
}

impl<T: Debug + Clone> Iterator for Children<T> {
    type Item = Node<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let TrieNode::Node { ref children, .. } = self.trie {
            if self.idx_child < children.len() {
                let current_child = children[self.idx_child].clone();
                self.idx_child += 1;
                if let TrieNode::Node {
                    ref value,
                    ref prefix,
                    ..
                } = current_child.as_ref()
                {
                    return Some(Node {
                        prefix: prefix.clone(),
                        value: value.clone(),
                        trie_node: *current_child.clone(),
                    });
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trie_basic() {
        let mut trie = Trie::default();
        trie.insert("/".to_string(), "/");
        trie.insert("/vec".to_string(), "/");
        trie.insert("/json".to_string(), "/");

        let prefixes: Vec<Vec<u8>> = trie.children().map(|c| c.prefix.clone()).collect();
        assert_eq!(prefixes, vec![vec![47]])
    }
}
