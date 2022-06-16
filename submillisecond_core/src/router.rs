//! Modified version of [matchit](https://crates.io/crates/matchit).

use self::{error::MatchError, params::Params, tree::ConstNode};

pub mod error;
pub mod params;
pub mod tree;

/// A URL router.
///
/// See [the crate documentation](crate) for details.
#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct Router<'a, T> {
    root: ConstNode<'a, T>,
}

impl<T> Default for Router<'_, T> {
    fn default() -> Self {
        Self {
            root: ConstNode::default(),
        }
    }
}

impl<'a, T> Router<'a, T> {
    /// Construct a new router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a new router.
    pub const fn from_node(node: ConstNode<'a, T>) -> Self {
        Router { root: node }
    }

    /// Tries to find a value in the router matching the given path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use matchit::Router;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut router = Router::new();
    /// router.insert("/home", "Welcome!")?;
    ///
    /// let matched = router.at("/home").unwrap();
    /// assert_eq!(*matched.value, "Welcome!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn at<'m, 'p>(&'m self, path: &'p str) -> Result<Match<&'m T>, MatchError> {
        match self.root.at(path.as_bytes()) {
            Ok((value, params)) => Ok(Match { value, params }),
            Err(e) => Err(e),
        }
    }

    /// Performs a case-insensitive lookup of the given path,
    /// returning the case corrected path if successful.
    ///
    /// Missing/extra trailing slashes are also corrected.
    ///
    /// ```rust
    /// # use matchit::Router;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut router = Router::new();
    /// router.insert("/home", "Welcome!")?;
    ///
    /// let path = router.fix_path("/HoMe/").unwrap();
    /// assert_eq!(path, "/home");
    /// # Ok(())
    /// # }
    /// ````
    pub fn fix_path(&self, path: &str) -> Option<String> {
        self.root.fix_path(path)
    }

    #[cfg(feature = "__test_helpers")]
    pub fn check_priorities(&self) -> Result<u32, (u32, u32)> {
        self.root.check_priorities()
    }
}

/// A successful match consisting of the registered value
/// and URL parameters, returned by [`Router::at`](Router::at).
#[derive(Debug)]
pub struct Match<V> {
    /// The value stored under the matched node.
    pub value: V,
    /// The route parameters. See [parameters](crate#parameters) for more details.
    pub params: Params,
}
