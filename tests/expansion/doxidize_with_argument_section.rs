use doxidize::*;

#[doxidize]
/// this is documentation
/// and this is too
#[arguments_section]
/// this goes after the arguments section
fn foo(
    /// this has one line of docs
    bar: u32,
    /// this has
    /// two lines of docs
    baz: String,
    _undocumented: i32,
) -> bool {
    baz.len() > bar as usize
}
