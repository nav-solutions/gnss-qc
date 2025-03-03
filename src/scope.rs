use crate::prelude::ProductType;

#[derive(Debug, Clone, PartialEq)]
pub enum QcScope {
    /// Scope by [ProductType] (file type)
    ProductType(ProductType),
    /// Scope by file name
    FileName(String),
    /// Scope by Agency
    Agency(String),
    /// Scope by Operator / Observer
    Operator(String),
}
