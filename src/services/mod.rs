#[cfg(feature = "completion")]
pub(crate) mod html_completion;
#[cfg(feature = "folding")]
pub(crate) mod html_folding;
#[cfg(feature = "formatter")]
pub(crate) mod html_formatter;
#[cfg(feature = "highlight")]
pub(crate) mod html_highlight;
#[cfg(feature = "hover")]
pub(crate) mod html_hover;
#[cfg(feature = "linked_editing")]
pub(crate) mod html_linked_editing;
#[cfg(feature = "links")]
pub(crate) mod html_links;
#[cfg(feature = "matching_tag_position")]
pub(crate) mod html_matching_tag_position;
pub(crate) mod html_rename;
pub(crate) mod html_selection_range;
pub(crate) mod html_symbols;
