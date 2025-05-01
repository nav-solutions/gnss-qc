/// [SolutionsIter] is implemented by all our navigation solvers,
/// to efficiently resolve and grab the solutions at a specific point in time.
use gnss_rtk::prelude::User;

#[cfg(doc)]
use gnss_rtk::prelude::Epoch;

pub trait SolutionsIter {
    /// The type of solution this solver will stream.   
    /// In our framework, it's either P.V.T solutions or CGGTTS solutions
    /// (P.V.T solutions through a special fit algorithm).
    type Solution;

    /// The error type returned when the solving process goes wrong.   
    /// In our framework, it's either the GNSS-RTK solver error, or
    /// a combination of both GNSS-RTK solver and CGGTTS solver when the
    /// special CGGTTS fit is also requested.
    type Error;

    /// Grab the next [PVTSolution] is chronological order.   
    /// When the pending [Epoch] results in solver failure, Some([Error]) is returned.   
    /// When the last [PVTSolution] has been resolved, the iterator returns [None],
    /// as per standards Rust Iterator.
    /// ## Input
    /// - mutable [Self]
    /// - user_profile: latest [User] profile that describes the rover.
    fn next(&mut self, user_profile: User) -> Option<Result<Self::Solution, Self::Error>>;
}
