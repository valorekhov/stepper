//! Utility module for miscellaneous stuff that the rest of the crate needs

pub mod delay;
pub mod ref_mut;

#[macro_export]
macro_rules! pin_mut {
    ($($x:ident),* $(,)?) => { $(
        // Move the value to ensure that it is owned
        let mut $x = $x;
        // Shadow the original binding so that it can't be directly accessed
        // ever again.
        #[allow(unused_mut)]
        let mut $x = unsafe {
            $crate::core::pin::Pin::new_unchecked(&mut $x)
        };
    )* }
}
