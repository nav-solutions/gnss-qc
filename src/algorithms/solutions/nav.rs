/// Navigation solutions for a specific receiver.
pub struct NavigationSolutions {
    /// Name of the targetted receiver, based off the user preferences
    /// and provided context.
    pub name: String,

    /// [CGGTTS] solutions, obtained by solving [Algorithms::CggttsSolutions]
    pub solutions: CGGTTS,
}
