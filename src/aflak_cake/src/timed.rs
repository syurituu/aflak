use std::fmt;
use std::ops::Deref;
use std::time::Instant;

/// A smart pointer embedding the instant the contained value was created.
///
/// # Example
///
/// ```rust
/// extern crate aflak_cake;
/// use aflak_cake::Timed;
///
/// let timed_value = Timed::from(0);
/// // Do stuff with timed value
/// println!("{}", *timed_value + 1);
/// // Print the instant on which the value was created on the line above
/// println!("{:?}", Timed::created_on(&timed_value));
/// ```
#[derive(Debug, Copy, Clone)]
pub struct Timed<T> {
    value: T,
    created_on: Instant,
}

impl<T: fmt::Display> fmt::Display for Timed<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} (updated {:?} ago)",
            self.value,
            self.created_on.elapsed()
        )
    }
}

impl<T> Deref for Timed<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> Timed<T> {
    /// Create a new value assumed to have been created at instant `created_on`.
    pub fn from_instant(t: T, created_on: Instant) -> Self {
        Self {
            value: t,
            created_on,
        }
    }

    /// Extract contained value.
    pub fn take(t: Self) -> T {
        t.value
    }

    /// Get instant on which the value was created.
    pub fn created_on(t: &Self) -> Instant {
        t.created_on
    }

    /// Extract values from a Result of timed values.
    pub fn take_from_result<E>(result: Result<Timed<T>, Timed<E>>) -> Result<T, E> {
        result.map(Timed::take).map_err(Timed::take)
    }

    /// Change the contained value by applying the function `f`. This function
    /// does not alter the instant the value was created.
    pub fn map<U, F: FnOnce(T) -> U>(t: Timed<T>, f: F) -> Timed<U> {
        Timed {
            value: f(t.value),
            created_on: t.created_on,
        }
    }
}

impl<T, E> Timed<Result<T, E>> {
    /// Convert a timed result to a result of times values.
    pub fn map_result(result: Timed<Result<T, E>>) -> Result<Timed<T>, Timed<E>> {
        let created_on = result.created_on;
        match result.value {
            Ok(t) => Ok(Timed::from_instant(t, created_on)),
            Err(e) => Err(Timed::from_instant(e, created_on)),
        }
    }
}

impl<T> From<T> for Timed<T> {
    fn from(t: T) -> Self {
        Self {
            value: t,
            created_on: Instant::now(),
        }
    }
}
