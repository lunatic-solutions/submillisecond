//! Params are data from the request url.
//!
//! The [`router!`](crate::router) macro collects params into the
//! [`crate::RequestContext`]

use std::{iter, mem, slice};

/// A single URL parameter, consisting of a key and a value.
#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Default, Clone)]
pub struct Param {
    /// Param key as defined in the url.
    pub key: &'static str,
    /// Param value.
    pub value: String,
}

/// A list of parameters returned by a route match.
///
/// # Extractor example
///
/// ```
/// use submillisecond::router;
/// use submillisecond::params::Params;
///
/// fn params(params: Params) -> String {
///     let name = params.get("name").unwrap_or("user");
///     format!("Welcome, {name}")
/// }
///
/// router! {
///     GET "/:name" => params
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub struct Params {
    kind: ParamsKind,
}

// Most routes have 1-3 dynamic parameters, so we can avoid a heap allocation in
// common cases.
const SMALL: usize = 3;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Clone)]
enum ParamsKind {
    None,
    Small([Param; SMALL], usize),
    Large(Vec<Param>),
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}

impl Params {
    /// Creates an empty instance of [`Params`].
    pub fn new() -> Self {
        let kind = ParamsKind::None;
        Self { kind }
    }

    /// Returns the number of parameters.
    pub fn len(&self) -> usize {
        match &self.kind {
            ParamsKind::None => 0,
            ParamsKind::Small(_, len) => *len,
            ParamsKind::Large(vec) => vec.len(),
        }
    }

    /// Returns the value of the first parameter registered under the given key.
    pub fn get(&self, key: &str) -> Option<&str> {
        match &self.kind {
            ParamsKind::None => None,
            ParamsKind::Small(arr, len) => arr
                .iter()
                .take(*len)
                .find(|param| param.key == key)
                .map(|value| value.value.as_str()),
            ParamsKind::Large(vec) => vec
                .iter()
                .find(|param| param.key == key)
                .map(|value| value.value.as_str()),
        }
    }

    /// Returns an iterator over the parameters in the list.
    pub fn iter(&self) -> ParamsIter<'_> {
        ParamsIter::new(self)
    }

    /// Returns `true` if there are no parameters in the list.
    pub fn is_empty(&self) -> bool {
        match &self.kind {
            ParamsKind::None => true,
            ParamsKind::Small(_, len) => *len == 0,
            ParamsKind::Large(vec) => vec.is_empty(),
        }
    }

    /// Inserts a key value parameter pair into the list.
    pub fn push(&mut self, key: &'static str, value: String) {
        #[cold]
        fn drain_to_vec<T: Default>(len: usize, elem: T, arr: &mut [T; SMALL]) -> Vec<T> {
            let mut vec = Vec::with_capacity(len + 1);
            vec.extend(arr.iter_mut().map(mem::take));
            vec.push(elem);
            vec
        }

        let param = Param { key, value };
        match &mut self.kind {
            ParamsKind::None => {
                self.kind = ParamsKind::Small([param, Param::default(), Param::default()], 1);
            }
            ParamsKind::Small(arr, len) => {
                if *len == SMALL {
                    self.kind = ParamsKind::Large(drain_to_vec(*len, param, arr));
                    return;
                }
                arr[*len] = param;
                *len += 1;
            }
            ParamsKind::Large(vec) => vec.push(param),
        }
    }
}

/// An iterator over the keys and values of a route's [parameters](Params).
pub struct ParamsIter<'ps> {
    kind: ParamsIterKind<'ps>,
}

impl<'ps> ParamsIter<'ps> {
    fn new(params: &'ps Params) -> Self {
        let kind = match &params.kind {
            ParamsKind::None => ParamsIterKind::None,
            ParamsKind::Small(arr, len) => ParamsIterKind::Small(arr.iter().take(*len)),
            ParamsKind::Large(vec) => ParamsIterKind::Large(vec.iter()),
        };
        Self { kind }
    }
}

enum ParamsIterKind<'ps> {
    None,
    Small(iter::Take<slice::Iter<'ps, Param>>),
    Large(slice::Iter<'ps, Param>),
}

impl<'ps> Iterator for ParamsIter<'ps> {
    type Item = (&'ps str, &'ps str);

    fn next(&mut self) -> Option<Self::Item> {
        match self.kind {
            ParamsIterKind::None => None,
            ParamsIterKind::Small(ref mut iter) => iter.next().map(|p| (p.key, p.value.as_str())),
            ParamsIterKind::Large(ref mut iter) => iter.next().map(|p| (p.key, p.value.as_str())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_alloc() {
        assert_eq!(Params::new().kind, ParamsKind::None);
    }

    #[test]
    fn heap_alloc() {
        let vec = vec![
            ("hello", "hello"),
            ("world", "world"),
            ("foo", "foo"),
            ("bar", "bar"),
            ("baz", "baz"),
        ];

        let mut params = Params::new();
        for (key, value) in vec.clone() {
            params.push(key, value.to_string());
            assert_eq!(params.get(key), Some(value));
        }

        match params.kind {
            ParamsKind::Large(..) => {}
            _ => panic!(),
        }

        assert!(params.iter().eq(vec.clone()));
    }

    #[test]
    fn stack_alloc() {
        let vec = vec![("hello", "hello"), ("world", "world"), ("baz", "baz")];

        let mut params = Params::new();
        for (key, value) in vec.clone() {
            params.push(key, value.to_string());
            assert_eq!(params.get(key), Some(value));
        }

        match params.kind {
            ParamsKind::Small(..) => {}
            _ => panic!(),
        }

        assert!(params.iter().eq(vec.clone()));
    }

    #[test]
    fn ignore_array_default() {
        let params = Params::new();
        assert!(params.get("").is_none());
    }
}
