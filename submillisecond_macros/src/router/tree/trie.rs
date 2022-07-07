use std::{fmt::Debug, mem};

#[derive(Debug)]
pub struct Trie<V> {
    node: TrieNode<V>,
}

impl<V> Default for Trie<V> {
    fn default() -> Self {
        Trie {
            node: TrieNode::Empty,
        }
    }
}

impl<V> Trie<V>
where
    V: Clone,
{
    pub fn children(&self) -> Children<V> {
        Children::new(TrieNode::Node {
            value: None,
            prefix: String::new(),
            children: vec![self.node.clone()],
        })
    }

    pub fn insert(&mut self, key: String, new_value: V) {
        self.node.insert(key, new_value)
    }
}

#[derive(Debug, Clone)]
pub enum TrieNode<V> {
    Node {
        value: Option<V>,
        prefix: String,
        children: Vec<TrieNode<V>>,
    },
    Empty,
}

impl<V> TrieNode<V>
where
    V: Clone,
{
    pub fn create_terminal(prefix: String, value: V) -> Self {
        TrieNode::Node {
            value: Some(value),
            prefix,
            children: vec![],
        }
    }

    pub fn insert(&mut self, key: String, new_value: V) {
        let key_len = key.len();
        match self {
            TrieNode::Node {
                ref mut prefix,
                ref mut children,
                ref mut value,
            } => {
                let mut last_match = key_len;
                for (idx, b) in key.chars().enumerate() {
                    // if node prefix ended, try to delegate to a child
                    if idx >= prefix.len() {
                        return Self::delegate_to_child(
                            key[idx..].to_string(),
                            new_value,
                            children,
                        );
                    }
                    // if matches current node, delegate to a child
                    if b == prefix.chars().nth(idx).unwrap() {
                        continue;
                    } else {
                        // they are not the same, need to split at longest common prefix
                        last_match = idx;
                        break;
                    }
                }
                // inserting the same key
                if last_match == prefix.len() {
                    return;
                }
                // in this case, key_len will ALWAYS be shorter than prefix.len()
                // prefix has left-over data, need to split the prefix
                let (new_prefix, suffix) = prefix.split_at(last_match);
                // create new node that carries data from current node
                let new_child = TrieNode::Node {
                    value: mem::take(value),
                    prefix: suffix.to_string(),
                    children: vec![],
                };
                // insert new node with new suffix if
                if last_match < key_len {
                    // create new self
                    *self = TrieNode::Node {
                        value: None,
                        prefix: new_prefix.to_string(),
                        children: vec![
                            new_child,
                            TrieNode::Node {
                                value: Some(new_value),
                                prefix: key.as_str()[last_match..].to_string(),
                                children: vec![],
                            },
                        ],
                    };
                } else {
                    // create new self
                    *self = TrieNode::Node {
                        value: Some(new_value),
                        prefix: new_prefix.to_string(),
                        children: vec![new_child],
                    };
                }
            }
            TrieNode::Empty => {
                *self = TrieNode::create_terminal(key, new_value);
            }
        }
    }

    fn delegate_to_child(key: String, new_value: V, children: &mut Vec<TrieNode<V>>) {
        let next = key.chars().next();
        // if we find any existing match for the next one we pass it on
        let child = children.iter_mut().find(|c| {
            if let TrieNode::Node { prefix, .. } = c {
                // if first character matches we found the right child
                return prefix.chars().next() == next;
            }
            false
        });
        if let Some(child) = child {
            child.insert(key, new_value);
        } else {
            // create a new terminal child
            children.push(TrieNode::create_terminal(key, new_value));
        }
    }

    pub fn children(&self) -> Children<V> {
        Children::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Node<V> {
    pub prefix: String,
    pub value: Option<V>,
    pub trie_node: TrieNode<V>,
}

impl<V> Node<V>
where
    V: Clone,
{
    pub fn children(&self) -> Children<V> {
        self.trie_node.children()
    }

    pub fn is_leaf(&self) -> bool {
        if let TrieNode::Node { ref children, .. } = self.trie_node {
            return children.is_empty();
        }
        true
    }
}

#[derive(Clone, Debug)]
pub struct Children<V>
where
    V: Clone,
{
    trie: TrieNode<V>,
    idx_child: usize,
}

impl<V> Children<V>
where
    V: Clone,
{
    pub fn new(trie: TrieNode<V>) -> Self {
        Children { trie, idx_child: 0 }
    }
}

impl<V> Iterator for Children<V>
where
    V: Clone,
{
    type Item = Node<V>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.trie {
            TrieNode::Node { ref children, .. } if self.idx_child < children.len() => {
                let current_child = &children[self.idx_child];
                self.idx_child += 1;
                if let TrieNode::Node { value, prefix, .. } = current_child {
                    Some(Node {
                        prefix: prefix.clone(),
                        value: value.clone(),
                        trie_node: current_child.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
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

        let prefixes: Vec<_> = trie.children().map(|c| c.prefix).collect();
        assert_eq!(prefixes, vec!["/"])
    }
}
