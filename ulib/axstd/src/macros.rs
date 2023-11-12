//! Standard library macros

/// Prints to the standard output.
///
/// Equivalent to the [`println!`] macro except that a newline is not printed at
/// the end of the message.
///
/// [`println!`]: crate::println
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::io::__print_impl(format_args!($($arg)*));
    }
}

/// Prints to the standard output, with a newline.
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {
        let color = 92;
        $crate::io::__print_impl(format_args!("\u{1b}[{}m{}\u{1b}[0m\n", color, format_args!($($arg)*)));
    }
}

// /// Prints to the standard output, with color and a newline.
// #[macro_export]
// macro_rules! println_color {
//     () => { $crate::print!("\n") };
//     ($color:expr, $($arg:tt)*) => {
//         $crate::io::__print_impl(format_args!("\u{1b}[{}m{}\u{1b}[0m\n", $color, format_args!($($arg)*)));
//     }
// }
