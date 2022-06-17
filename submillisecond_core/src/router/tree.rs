use std::cmp::min;
use std::{mem, str};

use super::error::{InsertError, MatchError};
use super::params::Params;

/// The types of nodes the tree can hold
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum NodeType {
    /// The root path
    Root,
    /// A route parameter, ex: `/:id`.
    Param,
    /// A catchall parameter, ex: `/*file`
    CatchAll,
    /// Anything else
    Static,
}

/// A radix tree used for URL path matching.
///
/// See [the crate documentation](crate) for details.
pub struct Node<T> {
    pub priority: u32,
    pub wild_child: bool,
    pub indices: Vec<u8>,
    pub node_type: NodeType,
    // see `at_inner` for why an unsafe cell is needed.
    pub value: Vec<T>,
    pub prefix: Vec<u8>,
    pub children: Vec<Self>,
}

/// A [Node] for initializing as const.
pub struct ConstNode<'a, T> {
    pub priority: u32,
    pub wild_child: bool,
    pub indices: &'a [u8],
    pub node_type: NodeType,
    pub value: &'a [T],
    pub prefix: &'a [u8],
    pub children: &'a [Self],
}

// SAFETY: we expose `value` per rust's usual borrowing rules, so we can just delegate these traits
unsafe impl<T: Send> Send for Node<T> {}
unsafe impl<T: Sync> Sync for Node<T> {}

unsafe impl<T: Send> Send for ConstNode<'_, T> {}
unsafe impl<T: Sync> Sync for ConstNode<'_, T> {}

impl<T> Clone for Node<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            wild_child: self.wild_child,
            node_type: self.node_type,
            indices: self.indices.clone(),
            children: self.children.clone(),
            // SAFETY: we only expose &mut T through &mut self
            value: self.value.clone(),
            priority: self.priority,
        }
    }
}

impl<T> Clone for ConstNode<'_, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix,
            wild_child: self.wild_child,
            node_type: self.node_type,
            indices: self.indices,
            children: self.children,
            // SAFETY: we only expose &mut T through &mut self
            value: self.value,
            priority: self.priority,
        }
    }
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self {
            prefix: Vec::new(),
            wild_child: false,
            node_type: NodeType::Static,
            indices: Vec::new(),
            children: Vec::new(),
            value: Vec::new(),
            priority: 0,
        }
    }
}

impl<T> Default for ConstNode<'_, T> {
    fn default() -> Self {
        Self {
            prefix: &[],
            wild_child: false,
            node_type: NodeType::Static,
            indices: &[],
            children: &[],
            value: &[],
            priority: 0,
        }
    }
}

impl<T: Clone> Node<T> {
    pub fn insert(&mut self, route: impl Into<String>, val: T) -> Result<(), InsertError> {
        let route = route.into().into_bytes();
        let mut prefix = route.as_ref();

        self.priority += 1;

        // empty tree
        if self.prefix.is_empty() && self.children.is_empty() {
            self.insert_child(prefix, &route, val)?;
            self.node_type = NodeType::Root;
            return Ok(());
        }

        let mut current = self;

        'walk: loop {
            // find the longest common prefix
            //
            // this also implies that the common prefix contains
            // no ':' or '*', since the existing key can't contain
            // those chars
            let mut i = 0;
            let max = min(prefix.len(), current.prefix.len());

            while i < max && prefix[i] == current.prefix[i] {
                i += 1;
            }

            // split edge
            if i < current.prefix.len() {
                let mut child = Self {
                    prefix: current.prefix[i..].to_owned(),
                    wild_child: current.wild_child,
                    indices: current.indices.clone(),
                    value: current.value.clone(),
                    priority: current.priority - 1,
                    ..Self::default()
                };

                mem::swap(&mut current.children, &mut child.children);

                current.children = vec![child];
                current.indices = current.prefix[i..=i].to_owned();
                current.prefix = prefix[..i].to_owned();
                current.wild_child = false;
            }

            // make new node a child of this node
            if prefix.len() > i {
                prefix = &prefix[i..];

                let idxc = prefix[0];

                // `/` after param
                if current.node_type == NodeType::Param
                    && idxc == b'/'
                    && current.children.len() == 1
                {
                    current = &mut current.children[0];
                    current.priority += 1;

                    continue 'walk;
                }

                // check if a child with the next path byte exists
                for mut i in 0..current.indices.len() {
                    if idxc == current.indices[i] {
                        i = current.update_child_priority(i);
                        current = &mut current.children[i];
                        continue 'walk;
                    }
                }

                if idxc != b':' && idxc != b'*' && current.node_type != NodeType::CatchAll {
                    current.indices.push(idxc);
                    let mut child = current.add_child(Self::default());
                    child = current.update_child_priority(child);
                    current = &mut current.children[child];
                } else if current.wild_child {
                    // inserting a wildcard node, check if it conflicts with the existing wildcard
                    current = current.children.last_mut().unwrap();
                    current.priority += 1;

                    // check if the wildcard matches
                    if prefix.len() >= current.prefix.len()
                        && current.prefix == prefix[..current.prefix.len()]
                        // adding a child to a catchall Node is not possible
                        && current.node_type != NodeType::CatchAll
                        // check for longer wildcard, e.g. :name and :names
                        && (current.prefix.len() >= prefix.len()
                            || prefix[current.prefix.len()] == b'/')
                    {
                        continue 'walk;
                    }

                    return Err(InsertError::conflict(&route, prefix, current));
                }

                return current.insert_child(prefix, &route, val);
            }

            // otherwise add value to current node
            current.value.push(val);

            return Ok(());
        }
    }

    // add a child node, keeping wildcards at the end
    fn add_child(&mut self, child: Node<T>) -> usize {
        let len = self.children.len();

        if self.wild_child && len > 0 {
            self.children.insert(len - 1, child);
            len - 1
        } else {
            self.children.push(child);
            len
        }
    }

    // increments priority of the given child and reorders if necessary
    // returns the new position (index) of the child
    fn update_child_priority(&mut self, pos: usize) -> usize {
        self.children[pos].priority += 1;
        let prio = self.children[pos].priority;
        // adjust position (move to front)
        let mut new_pos = pos;

        while new_pos > 0 && self.children[new_pos - 1].priority < prio {
            // swap node positions
            self.children.swap(new_pos - 1, new_pos);
            new_pos -= 1;
        }

        // build new index char string
        if new_pos != pos {
            self.indices = [
                &self.indices[..new_pos],    // unchanged prefix, might be empty
                &self.indices[pos..=pos],    // the index char we move
                &self.indices[new_pos..pos], // rest without char at 'pos'
                &self.indices[pos + 1..],
            ]
            .concat();
        }

        new_pos
    }

    fn insert_child(&mut self, mut prefix: &[u8], route: &[u8], val: T) -> Result<(), InsertError> {
        let mut current = self;

        loop {
            // search for a wildcard segment
            let (wildcard, wildcard_index) = match find_wildcard(prefix) {
                (Some((w, i)), true) => (w, i),
                // the wildcard name contains invalid characters (':' or '*')
                (Some(..), false) => return Err(InsertError::TooManyParams),
                // no wildcard, simply use the current node
                (None, _) => {
                    current.value.push(val);
                    current.prefix = prefix.to_owned();
                    return Ok(());
                }
            };

            // check if the wildcard has a name
            if wildcard.len() < 2 {
                return Err(InsertError::UnnamedParam);
            }

            // route parameter
            if wildcard[0] == b':' {
                // insert prefix before the current wildcard
                if wildcard_index > 0 {
                    current.prefix = prefix[..wildcard_index].to_owned();
                    prefix = &prefix[wildcard_index..];
                }

                let child = Self {
                    node_type: NodeType::Param,
                    prefix: wildcard.to_owned(),
                    ..Self::default()
                };

                let child = current.add_child(child);
                current.wild_child = true;
                current = &mut current.children[child];
                current.priority += 1;

                // if the route doesn't end with the wildcard, then there
                // will be another non-wildcard subroute starting with '/'
                if wildcard.len() < prefix.len() {
                    prefix = &prefix[wildcard.len()..];
                    let child = Self {
                        priority: 1,
                        ..Self::default()
                    };

                    let child = current.add_child(child);
                    current = &mut current.children[child];
                    continue;
                }

                // otherwise we're done. Insert the value in the new leaf
                current.value.push(val);
                return Ok(());
            }

            // catch all route
            assert_eq!(wildcard[0], b'*');

            // "/foo/*catchall/bar"
            if wildcard_index + wildcard.len() != prefix.len() {
                return Err(InsertError::InvalidCatchAll);
            }

            if let Some(i) = wildcard_index.checked_sub(1) {
                // "/foo/bar*catchall"
                if prefix[i] != b'/' {
                    return Err(InsertError::InvalidCatchAll);
                }
            }

            // "*catchall"
            if prefix == route && route[0] != b'/' {
                return Err(InsertError::InvalidCatchAll);
            }

            if wildcard_index > 0 {
                current.prefix = prefix[..wildcard_index].to_owned();
                prefix = &prefix[wildcard_index..];
            }

            let child = Self {
                prefix: prefix.to_owned(),
                node_type: NodeType::CatchAll,
                value: vec![val],
                priority: 1,
                ..Self::default()
            };

            current.add_child(child);
            current.wild_child = true;

            return Ok(());
        }
    }
}

struct Skipped<'n, 'p, 'a, T> {
    path: &'p [u8],
    node: &'n ConstNode<'a, T>,
    params: usize,
}

#[rustfmt::skip]
macro_rules! backtracker {
    ($skipped_nodes:ident, $path:ident, $current:ident, $params:ident, $backtracking:ident, $walk:lifetime) => {
        macro_rules! try_backtrack {
            () => {
                while let Some(skipped) = $skipped_nodes.pop() {
                    if skipped.path.ends_with($path) {
                        $path = skipped.path;
                        $current = &skipped.node;
                        $params.truncate(skipped.params);

                        $backtracking = true;
                        continue $walk;
                    }
                }
            };
        }
    };
}

impl<T> ConstNode<'_, T> {
    // It's a bit sad that we have to introduce unsafe here,
    // but rust doesn't really have a way to abstract over mutability,
    // so it avoids having to duplicate logic between `at` and `at_mut`.
    pub fn at<'n, 'p>(&'n self, path: &'p [u8]) -> Result<(&'n [T], Params), MatchError> {
        let full_path = path;

        let mut current = self;
        let mut path = full_path;
        let mut backtracking = false;
        let mut params = Params::new();
        let mut skipped_nodes: Vec<Skipped<'_, '_, '_, _>> = Vec::new();

        'walk: loop {
            backtracker!(skipped_nodes, path, current, params, backtracking, 'walk);

            if path.len() > current.prefix.len() {
                let (prefix, rest) = path.split_at(current.prefix.len());

                if prefix == current.prefix {
                    path = rest;
                    let index = path[0];

                    // try all the non-wildcard children first by matching
                    // the indices, unless we are currently backtracking
                    if !backtracking {
                        if let Some(i) = current.indices.iter().position(|&c| c == index) {
                            if current.wild_child {
                                skipped_nodes.push(Skipped {
                                    path: &full_path
                                        [full_path.len() - (current.prefix.len() + path.len())..],
                                    node: current,
                                    params: params.len(),
                                });
                            }

                            current = &current.children[i];
                            continue 'walk;
                        }
                    }

                    if !current.wild_child {
                        if path == b"/" && !current.value.is_empty() {
                            return Err(MatchError::ExtraTrailingSlash);
                        }

                        // try backtracking
                        if path != b"/" {
                            try_backtrack!();
                        }

                        // nothing found
                        return Err(MatchError::NotFound);
                    }

                    // handle wildcard child, which is always at the end of the array
                    current = current.children.last().unwrap();

                    match current.node_type {
                        NodeType::Param => {
                            match path.iter().position(|&c| c == b'/') {
                                Some(param_idx) => {
                                    let (param, rest) = path.split_at(param_idx);
                                    params.push(
                                        String::from_utf8(current.prefix[1..].to_vec()).unwrap(),
                                        String::from_utf8(param.to_vec()).unwrap(),
                                    );

                                    if current.children.is_empty() {
                                        if path.len() == param_idx + 1 {
                                            return Err(MatchError::ExtraTrailingSlash);
                                        }

                                        return Err(MatchError::NotFound);
                                    }

                                    path = rest;
                                    current = &current.children[0];

                                    backtracking = false;
                                    continue 'walk;
                                }
                                None => {
                                    params.push(
                                        String::from_utf8(current.prefix[1..].to_vec()).unwrap(),
                                        String::from_utf8(path.to_vec()).unwrap(),
                                    );
                                }
                            }

                            if !current.value.is_empty() {
                                return Ok((current.value, params));
                            }

                            if let [only_child] = current.children {
                                current = only_child;

                                if (current.prefix == b"/" && !current.value.is_empty())
                                    || (current.prefix.is_empty() && current.indices == b"/")
                                {
                                    return Err(MatchError::MissingTrailingSlash);
                                }

                                if path != b"/" {
                                    try_backtrack!();
                                }
                            }

                            return Err(MatchError::NotFound);
                        }
                        NodeType::CatchAll => {
                            params.push(
                                String::from_utf8(current.prefix[1..].to_vec()).unwrap(),
                                String::from_utf8(path.to_vec()).unwrap(),
                            );

                            return if !current.value.is_empty() {
                                Ok((current.value, params))
                            } else {
                                Err(MatchError::NotFound)
                            };
                        }
                        _ => unreachable!(),
                    }
                }
            }

            if path == current.prefix {
                // we should have reached the node containing the value
                if !current.value.is_empty() {
                    return Ok((current.value, params));
                }

                // otherwise try backtracking
                if path != b"/" {
                    try_backtrack!();
                }

                // if there is no value for this route, but this route has a
                // wildcard child, there must be a handle for this path with an
                // additional trailing slash
                if path == b"/" && current.wild_child && current.node_type != NodeType::Root {
                    // TODO: this case is also being triggered when there is an overlap
                    // of dynamic and static route segments and an *extra* trailing slash
                    return Err(MatchError::unsure(full_path));
                }

                // check if the path is missing a trailing '/'
                if !backtracking {
                    if let Some(i) = current.indices.iter().position(|&c| c == b'/') {
                        current = &current.children[i];

                        if current.prefix.len() == 1 && !current.value.is_empty() {
                            return Err(MatchError::MissingTrailingSlash);
                        }
                    }
                }

                return Err(MatchError::NotFound);
            }

            if path == b"/" && full_path != b"/" {
                return Err(MatchError::ExtraTrailingSlash);
            }

            if current.prefix.split_last() == Some((&b'/', path)) && !current.value.is_empty() {
                return Err(MatchError::MissingTrailingSlash);
            }

            // if there is no tsr, try backtracking
            if path != b"/" {
                try_backtrack!();
            }

            return Err(MatchError::NotFound);
        }
    }

    pub fn fix_path(&self, path: &str) -> Option<String> {
        let mut insensitive_path = Vec::with_capacity(path.len() + 1);
        let found = self.fix_path_helper(path.as_bytes(), &mut insensitive_path, [0; 4]);
        if found {
            Some(String::from_utf8(insensitive_path).unwrap())
        } else {
            None
        }
    }

    fn fix_path_helper(
        &self,
        mut path: &[u8],
        insensitive_path: &mut Vec<u8>,
        mut buf: [u8; 4],
    ) -> bool {
        let lower_path: &[u8] = &path.to_ascii_lowercase();
        if lower_path.len() >= self.prefix.len()
            && (self.prefix.is_empty()
                || lower_path[1..self.prefix.len()].eq_ignore_ascii_case(&self.prefix[1..]))
        {
            insensitive_path.extend_from_slice(self.prefix);

            path = &path[self.prefix.len()..];

            if !path.is_empty() {
                let cached_lower_path = <&[u8]>::clone(&lower_path);

                // if this node does not have a wildcard (param or catchAll) child,
                // we can just look up the next child node and continue to walk down
                // the tree
                if !self.wild_child {
                    // skip char bytes already processed
                    buf = shift_n_bytes(buf, self.prefix.len());

                    if buf[0] == 0 {
                        // process a new char
                        let mut current_char = 0 as char;

                        // find char start
                        // chars are up to 4 byte long,
                        // -4 would definitely be another char
                        let mut off = 0;
                        for j in 0..min(self.prefix.len(), 3) {
                            let i = self.prefix.len() - j;
                            if char_start(cached_lower_path[i]) {
                                // read char from cached path
                                current_char = str::from_utf8(&cached_lower_path[i..])
                                    .unwrap()
                                    .chars()
                                    .next()
                                    .unwrap();
                                off = j;
                                break;
                            }
                        }

                        current_char.encode_utf8(&mut buf);

                        // skip already processed bytes
                        buf = shift_n_bytes(buf, off);

                        for i in 0..self.indices.len() {
                            // lowercase matches
                            if self.indices[i] == buf[0] {
                                // must use a recursive approach since both the
                                // uppercase byte and the lowercase byte might exist
                                // as an index
                                if self.children[i].fix_path_helper(path, insensitive_path, buf) {
                                    return true;
                                }

                                if insensitive_path.len() > self.children[i].prefix.len() {
                                    let prev_len =
                                        insensitive_path.len() - self.children[i].prefix.len();
                                    insensitive_path.truncate(prev_len);
                                }

                                break;
                            }
                        }

                        // same for uppercase char, if it differs
                        let up = current_char.to_ascii_uppercase();
                        if up != current_char {
                            up.encode_utf8(&mut buf);
                            buf = shift_n_bytes(buf, off);

                            for i in 0..self.indices.len() {
                                if self.indices[i] == buf[0] {
                                    return self.children[i].fix_path_helper(
                                        path,
                                        insensitive_path,
                                        buf,
                                    );
                                }
                            }
                        }
                    } else {
                        // old char not finished
                        for i in 0..self.indices.len() {
                            if self.indices[i] == buf[0] {
                                // continue with child node
                                return self.children[i].fix_path_helper(
                                    path,
                                    insensitive_path,
                                    buf,
                                );
                            }
                        }
                    }

                    // nothing found. we can recommend to redirect to the same URL
                    // without a trailing slash if a leaf exists for that path
                    return path == [b'/'] && !self.value.is_empty();
                }

                return self.children[0].fix_path_match_helper(path, insensitive_path, buf);
            }

            // we should have reached the node containing the value.
            // check if this node has a value registered.
            if !self.value.is_empty() {
                return true;
            }

            // no value found.
            // try to fix the path by adding a trailing slash
            for i in 0..self.indices.len() {
                if self.indices[i] == b'/' {
                    if (self.children[i].prefix.len() == 1 && !self.children[i].value.is_empty())
                        || (self.children[i].node_type == NodeType::CatchAll
                            && !self.children[i].children[0].value.is_empty())
                    {
                        insensitive_path.push(b'/');
                        return true;
                    }
                    return false;
                }
            }

            return false;
        }

        // nothing found.
        // try to fix the path by adding / removing a trailing slash
        if path == [b'/'] {
            return true;
        }
        if lower_path.len() + 1 == self.prefix.len()
            && self.prefix[lower_path.len()] == b'/'
            && lower_path[1..].eq_ignore_ascii_case(&self.prefix[1..lower_path.len()])
            && !self.value.is_empty()
        {
            insensitive_path.extend_from_slice(self.prefix);
            return true;
        }

        false
    }

    fn fix_path_match_helper(
        &self,
        mut path: &[u8],
        insensitive_path: &mut Vec<u8>,
        buf: [u8; 4],
    ) -> bool {
        match self.node_type {
            NodeType::Param => {
                let mut end = 0;

                while end < path.len() && path[end] != b'/' {
                    end += 1;
                }

                insensitive_path.extend_from_slice(&path[..end]);

                if end < path.len() {
                    if !self.children.is_empty() {
                        path = &path[end..];

                        return self.children[0].fix_path_helper(path, insensitive_path, buf);
                    }

                    // ... but we can't
                    if path.len() == end + 1 {
                        return true;
                    }
                    return false;
                }

                if !self.value.is_empty() {
                    return true;
                } else if self.children.len() == 1
                    && self.children[0].prefix == [b'/']
                    && !self.children[0].value.is_empty()
                {
                    // no value found. check if a value for this path + a
                    // trailing slash exists
                    insensitive_path.push(b'/');
                    return true;
                }

                false
            }
            NodeType::CatchAll => {
                insensitive_path.extend_from_slice(path);
                true
            }
            _ => unreachable!(),
        }
    }

    #[cfg(feature = "__test_helpers")]
    pub fn check_priorities(&self) -> Result<u32, (u32, u32)> {
        let mut priority: u32 = 0;
        for child in &self.children {
            priority += child.check_priorities()?;
        }

        if self.value.is_some() {
            priority += 1;
        }

        if self.priority != priority {
            return Err((self.priority, priority));
        }

        Ok(priority)
    }
}

// Shift bytes in array by n bytes left
pub const fn shift_n_bytes(bytes: [u8; 4], n: usize) -> [u8; 4] {
    match u32::from_ne_bytes(bytes).overflowing_shr((n * 8) as u32) {
        (_, true) => [0; 4],
        (shifted, false) => shifted.to_ne_bytes(),
    }
}

// Reports whether the byte could be the first byte of a `char`.
const fn char_start(b: u8) -> bool {
    b & 0xC0 != 0x80
}

// Search for a wildcard segment and check the name for invalid characters.
fn find_wildcard(path: &[u8]) -> (Option<(&[u8], usize)>, bool) {
    for (start, &c) in path.iter().enumerate() {
        // a wildcard starts with ':' (param) or '*' (catch-all)
        if c != b':' && c != b'*' {
            continue;
        };

        // find end and check for invalid characters
        let mut valid = true;

        for (end, &c) in path[start + 1..].iter().enumerate() {
            match c {
                b'/' => return (Some((&path[start..start + 1 + end], start)), valid),
                b':' | b'*' => valid = false,
                _ => (),
            };
        }

        return (Some((&path[start..], start)), valid);
    }

    (None, false)
}

#[cfg(test)] // visualize the tree structure when debugging
impl<T: std::fmt::Debug> std::fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut x = f.debug_struct("Node");
        x.field("value", &self.value);
        x.field("prefix", &std::str::from_utf8(&self.prefix).unwrap());
        x.field("node_type", &self.node_type);
        x.field("children", &self.children);
        x.field(
            "indices",
            &self
                .indices
                .iter()
                .map(|&x| char::from_u32(x as _))
                .collect::<Vec<_>>(),
        );
        x.finish()
    }
}

#[cfg(test)] // visualize the tree structure when debugging
impl<T: std::fmt::Debug> std::fmt::Debug for ConstNode<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut x = f.debug_struct("Node");
        x.field("value", &self.value);
        x.field("prefix", &std::str::from_utf8(self.prefix).unwrap());
        x.field("node_type", &self.node_type);
        x.field("children", &self.children);
        x.field(
            "indices",
            &self
                .indices
                .iter()
                .map(|&x| char::from_u32(x as _))
                .collect::<Vec<_>>(),
        );
        x.finish()
    }
}
