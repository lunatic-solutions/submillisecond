//! Process local storage
//!
//! This implementation is mostly a copy of Rust's thread local implementation.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fmt;

/// A process local storage which owns its contents.
///
/// It is instantiated with the [`process_local!`] macro and the primary method
/// is the [`with`] method.
///
/// The [`with`] method yields a reference to the contained value which cannot
/// escape the given closure.
///
/// [`process_local!`]: crate::process_local!
///
/// # Initialization and Destruction
///
/// Initialization is dynamically performed on the first call to [`with`]
/// within a process, and values are **never** destructed. This means if a process
/// finishes normally or panics, the [`Drop`] implementation will never ba called.
///
/// A `ProcessLocal`'s initializer cannot recursively depend on itself, and using
/// a `ProcessLocal` in this way will cause the initializer to infinitely recurse
/// on the first call to `with`.
///
/// [`with`]: ProcessLocal::with
///
/// # Examples
///
/// ```
/// use lunatic::{process_local, spawn_link};
/// use std::cell::RefCell;
///
/// process_local!(static FOO: RefCell<u32> = RefCell::new(1));
///
/// FOO.with(|f| {
///     assert_eq!(*f.borrow(), 1);
///     *f.borrow_mut() = 2;
/// });
///
/// // each process starts out with the initial value of 1
/// let child = spawn_link!(@task || {
///     FOO.with(|f| {
///         assert_eq!(*f.borrow(), 1);
///         *f.borrow_mut() = 3;
///     });
/// });
///
/// // wait for the process to complete
/// let _ = child.result();
///
/// // we retain our original value of 2 despite the child process
/// FOO.with(|f| {
///     assert_eq!(*f.borrow(), 2);
/// });
/// ```
pub struct ProcessLocal<T: 'static> {
    // `*mut` is used instaed of `&mut` because mutable references are not
    // allowed in const functions: https://github.com/rust-lang/rust/issues/57349
    //
    // Although this is an extra layer of indirection, it should in theory be
    // trivially devirtualizable by LLVM because the value of `inner` never
    // changes and the constant should be readonly within a crate. This mainly
    // only runs into problems when PLS statics are exported across crates.
    inner: unsafe fn(Option<*mut Option<T>>) -> Option<&'static T>,
}

impl<T: 'static> fmt::Debug for ProcessLocal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessLocal").finish_non_exhaustive()
    }
}

/// Declare a new process local storage of type [`ProcessLocal`].
///
/// # Syntax
///
/// The macro wraps any number of static declarations and makes them process local.
/// Publicity and attributes for each static are allowed. Example:
///
/// ```
/// use std::cell::RefCell;
/// process_local! {
///     pub static FOO: RefCell<u32> = RefCell::new(1);
///
///     #[allow(unused)]
///     static BAR: RefCell<f32> = RefCell::new(1.0);
/// }
/// # fn main() {}
/// ```
///
/// See [`ProcessLocal` documentation][`$crate::ProcessLocal`] for more
/// information.
///
/// [`$crate::ProcessLocal`]: crate::ProcessLocal
#[macro_export]
macro_rules! process_local {
    // empty (base case for the recursion)
    () => {};

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const { $init:expr }; $($rest:tt)*) => (
        $crate::__process_local_inner!($(#[$attr])* $vis $name, $t, const $init);
        $crate::process_local!($($rest)*);
    );

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const { $init:expr }) => (
        $crate::__process_local_inner!($(#[$attr])* $vis $name, $t, const $init);
    );

    // process multiple declarations
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => (
        $crate::__process_local_inner!($(#[$attr])* $vis $name, $t, $init);
        $crate::process_local!($($rest)*);
    );

    // handle a single declaration
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => (
        $crate::__process_local_inner!($(#[$attr])* $vis $name, $t, $init);
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __process_local_inner {
    // used to generate the `ProcessLocal` value for const-initialized process locals
    (@key $t:ty, const $init:expr) => {{

        #[deny(unsafe_op_in_unsafe_fn)]
        unsafe fn __getit(
            _init: std::option::Option<&mut std::option::Option<$t>>,
        ) -> std::option::Option<&'static $t> {
            const INIT_EXPR: $t = $init;
            static mut VAL: $t = INIT_EXPR;
            unsafe { std::option::Option::Some(&VAL) }

        }

        unsafe {
            $crate::ProcessLocal::new(__getit)
        }
    }};

    // used to generate the `ProcessLocal` value for `process_local!`
    (@key $t:ty, $init:expr) => {
        {
            #[inline]
            fn __init() -> $t { $init }

            #[inline]
            unsafe fn __getit(
                init: std::option::Option<*mut std::option::Option<$t>>,
            ) -> std::option::Option<&'static $t> {
                static __KEY: lunatic::__StaticProcessLocalInner<$t> =
                    lunatic::__StaticProcessLocalInner::new();

                // FIXME: remove the #[allow(...)] marker when macros don't
                // raise warning for missing/extraneous unsafe blocks anymore.
                // See https://github.com/rust-lang/rust/issues/74838.
                #[allow(unused_unsafe)]
                unsafe {
                    __KEY.get(move || {
                        if let std::option::Option::Some(init) = init {
                            if let std::option::Option::Some(value) = init.as_mut().unwrap().take() {
                                return value;
                            } else if std::cfg!(debug_assertions) {
                                std::unreachable!("missing default value");
                            }
                        }
                        __init()
                    })
                }
            }

            unsafe {
                $crate::process_local::ProcessLocal::new(__getit)
            }
        }
    };
    ($(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $($init:tt)*) => {
        $(#[$attr])* $vis const $name: $crate::process_local::ProcessLocal<$t> =
            $crate::__process_local_inner!(@key $t, $($init)*);
    }
}

impl<T: 'static> ProcessLocal<T> {
    #[doc(hidden)]
    pub const unsafe fn new(
        inner: unsafe fn(Option<*mut Option<T>>) -> Option<&'static T>,
    ) -> ProcessLocal<T> {
        ProcessLocal { inner }
    }

    /// Acquires a reference to the value in this process local.
    ///
    /// This will lazily initialize the value if this process has not referenced
    /// it yet.
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&'static T) -> R,
    {
        unsafe {
            let process_local =
                (self.inner)(None).expect("Failed to access process local variable");
            f(process_local)
        }
    }

    /// Acquires a reference to the value in this process local, initializing
    /// it with `init` if it wasn't already initialized in this process.
    ///
    /// If `init` was used to initialize the process local variable, `None` is
    /// passed as the first argument to `f`. If it was already initialized,
    /// `Some(init)` is passed to `f`.
    fn initialize_with<F, R>(&'static self, init: T, f: F) -> R
    where
        F: FnOnce(Option<T>, &T) -> R,
    {
        unsafe {
            let mut init = Some(init);
            let reference =
                (self.inner)(Some(&mut init)).expect("Failed to access process local variable");
            f(init, reference)
        }
    }
}

impl<T: 'static> ProcessLocal<Cell<T>> {
    /// Sets or initializes the contained value.
    ///
    /// Unlike the other methods, this will *not* run the lazy initializer of
    /// the process local. Instead, it will be directly initialized with the
    /// given value if it wasn't initialized yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// process_local! {
    ///     static X: Cell<i32> = panic!("!");
    /// }
    ///
    /// // Calling X.get() here would result in a panic.
    ///
    /// X.set(123); // But X.set() is fine, as it skips the initializer above.
    ///
    /// assert_eq!(X.get(), 123);
    /// ```
    pub fn set(&'static self, value: T) {
        self.initialize_with(Cell::new(value), |value, cell| {
            if let Some(value) = value {
                // The cell was already initialized, so `value` wasn't used to
                // initialize it. So we overwrite the current value with the
                // new one instead.
                cell.set(value.into_inner());
            }
        });
    }

    /// Returns a copy of the contained value.
    ///
    /// This will lazily initialize the value if this process has not referenced
    /// it yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// process_local! {
    ///     static X: Cell<i32> = Cell::new(1);
    /// }
    ///
    /// assert_eq!(X.get(), 1);
    /// ```
    pub fn get(&'static self) -> T
    where
        T: Copy,
    {
        self.with(|cell| cell.get())
    }

    /// Takes the contained value, leaving `Default::default()` in its place.
    ///
    /// This will lazily initialize the value if this process has not referenced
    /// it yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// process_local! {
    ///     static X: Cell<Option<i32>> = Cell::new(Some(1));
    /// }
    ///
    /// assert_eq!(X.take(), Some(1));
    /// assert_eq!(X.take(), None);
    /// ```
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(|cell| cell.take())
    }

    /// Replaces the contained value, returning the old value.
    ///
    /// This will lazily initialize the value if this process has not referenced
    /// it yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// process_local! {
    ///     static X: Cell<i32> = Cell::new(1);
    /// }
    ///
    /// assert_eq!(X.replace(2), 1);
    /// assert_eq!(X.replace(3), 2);
    /// ```
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

impl<T: 'static> ProcessLocal<RefCell<T>> {
    /// Acquires a reference to the contained value.
    ///
    /// This will lazily initialize the value if this process has not referenced
    /// it yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    ///
    /// # Example
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// process_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// X.with_borrow(|v| assert!(v.is_empty()));
    /// ```
    pub fn with_borrow<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(Ref<'static, T>) -> R,
    {
        self.with(|cell| f(cell.borrow()))
    }

    /// Acquires a mutable reference to the contained value.
    ///
    /// This will lazily initialize the value if this process has not referenced
    /// it yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Example
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// process_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// X.with_borrow_mut(|v| v.push(1));
    ///
    /// X.with_borrow(|v| assert_eq!(*v, vec![1]));
    /// ```
    pub fn with_borrow_mut<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(RefMut<'static, T>) -> R,
    {
        self.with(|cell| f(cell.borrow_mut()))
    }

    /// Sets or initializes the contained value.
    ///
    /// Unlike the other methods, this will *not* run the lazy initializer of
    /// the process local. Instead, it will be directly initialized with the
    /// given value if it wasn't initialized yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// process_local! {
    ///     static X: RefCell<Vec<i32>> = panic!("!");
    /// }
    ///
    /// // Calling X.with() here would result in a panic.
    ///
    /// X.set(vec![1, 2, 3]); // But X.set() is fine, as it skips the initializer above.
    ///
    /// X.with_borrow(|v| assert_eq!(*v, vec![1, 2, 3]));
    /// ```
    pub fn set(&'static self, value: T) {
        self.initialize_with(RefCell::new(value), |value, cell| {
            if let Some(value) = value {
                // The cell was already initialized, so `value` wasn't used to
                // initialize it. So we overwrite the current value with the
                // new one instead.
                *cell.borrow_mut() = value.into_inner();
            }
        });
    }

    /// Takes the contained value, leaving `Default::default()` in its place.
    ///
    /// This will lazily initialize the value if this process has not referenced
    /// it yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// process_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// X.with_borrow_mut(|v| v.push(1));
    ///
    /// let a = X.take();
    ///
    /// assert_eq!(a, vec![1]);
    ///
    /// X.with_borrow(|v| assert!(v.is_empty()));
    /// ```
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(|cell| cell.take())
    }

    /// Replaces the contained value, returning the old value.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// process_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// let prev = X.replace(vec![1, 2, 3]);
    /// assert!(prev.is_empty());
    ///
    /// X.with_borrow(|v| assert_eq!(*v, vec![1, 2, 3]));
    /// ```
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

#[doc(hidden)]
#[allow(unused_unsafe)]
mod lazy {
    use std::cell::UnsafeCell;
    use std::hint;
    use std::mem;

    pub struct LazyKeyInner<T> {
        inner: UnsafeCell<Option<T>>,
    }

    impl<T> LazyKeyInner<T> {
        pub const fn new() -> LazyKeyInner<T> {
            LazyKeyInner {
                inner: UnsafeCell::new(None),
            }
        }

        pub unsafe fn get(&self) -> Option<&'static T> {
            // SAFETY: The caller must ensure no reference is ever handed out to
            // the inner cell nor mutable reference to the Option<T> inside said
            // cell. This make it safe to hand a reference, though the lifetime
            // of 'static is itself unsafe, making the get method unsafe.
            unsafe { (*self.inner.get()).as_ref() }
        }

        /// The caller must ensure that no reference is active: this method
        /// needs unique access.
        pub unsafe fn initialize<F: FnOnce() -> T>(&self, init: F) -> &'static T {
            // Execute the initialization up front, *then* move it into our slot,
            // just in case initialization fails.
            let value = init();
            let ptr = self.inner.get();

            // SAFETY:
            //
            // note that this can in theory just be `*ptr = Some(value)`, but due to
            // the compiler will currently codegen that pattern with something like:
            //
            //      ptr::drop_in_place(ptr)
            //      ptr::write(ptr, Some(value))
            //
            // Due to this pattern it's possible for the destructor of the value in
            // `ptr` (e.g., if this is being recursively initialized) to re-access
            // PLS, in which case there will be a `&` and `&mut` pointer to the same
            // value (an aliasing violation). To avoid setting the "I'm running a
            // destructor" flag we just use `mem::replace` which should sequence the
            // operations a little differently and make this safe to call.
            //
            // The precondition also ensures that we are the only one accessing
            // `self` at the moment so replacing is fine.
            unsafe {
                let _ = mem::replace(&mut *ptr, Some(value));
            }

            // SAFETY: With the call to `mem::replace` it is guaranteed there is
            // a `Some` behind `ptr`, not a `None` so `unreachable_unchecked`
            // will never be reached.
            unsafe {
                // After storing `Some` we want to get a reference to the contents of
                // what we just stored. While we could use `unwrap` here and it should
                // always work it empirically doesn't seem to always get optimized away,
                // which means that using something like `with` can pull in panicking
                // code and cause a large size bloat.
                match *ptr {
                    Some(ref x) => x,
                    None => hint::unreachable_unchecked(),
                }
            }
        }

        /// The other methods hand out references while taking &self.
        /// As such, callers of this method must ensure no `&` and `&mut` are
        /// available and used at the same time.
        #[allow(unused)]
        pub unsafe fn take(&mut self) -> Option<T> {
            // SAFETY: See doc comment for this method.
            unsafe { (*self.inner.get()).take() }
        }
    }
}

#[doc(hidden)]
#[allow(unused_unsafe)]
pub mod statik {
    use super::lazy::LazyKeyInner;
    use std::fmt;

    pub struct Key<T> {
        inner: LazyKeyInner<T>,
    }

    unsafe impl<T> Sync for Key<T> {}

    impl<T> fmt::Debug for Key<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Key").finish_non_exhaustive()
        }
    }

    impl<T> Key<T> {
        pub const fn new() -> Key<T> {
            Key {
                inner: LazyKeyInner::new(),
            }
        }

        pub unsafe fn get(&self, init: impl FnOnce() -> T) -> Option<&'static T> {
            // SAFETY: The caller must ensure no reference is ever handed out to
            // the inner cell nor mutable reference to the Option<T> inside said
            // cell. This make it safe to hand a reference, though the lifetime
            // of 'static is itself unsafe, making the get method unsafe.
            let value = unsafe {
                match self.inner.get() {
                    Some(value) => value,
                    None => self.inner.initialize(init),
                }
            };

            Some(value)
        }
    }
}
