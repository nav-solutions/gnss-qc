#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
mod html;

#[cfg(feature = "pdf")]
#[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
mod pdf;

#[cfg(feature = "pdf")]
#[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
mod plots;
