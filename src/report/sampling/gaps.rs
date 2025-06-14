#[derive(Default)]
pub struct QcSamplingHistogramAnalysis {
    /// Largest gap duration that was identified
    pub largest_gap: Duration,

    /// Shortest gap duration that was identified
    pub shortest_gap: Duration,

    /// Gap [Duration]s population
    pub gaps: HashMap<Duration, u64>,
}

impl QcSamplingHistogramAnalysis {
    pub fn latch(&mut self, gap: Duration) {
        if gap < self.shortest_gap {
            self.shortest_gap = gap;
        }

        if gap > self.longest_gap {
            self.longest_gap = gap;
        }

        if let Some(counts) = self.gap.get_mut(&gap) {
            *counts += 1;
        } else {
            self.gap.insert(gap, 1);
        }
    }
}

#[derive(Default, PartialEq, Eq, Hash)]
pub struct QcSamplingHistogramKey {
    pub indexing: QcIndexing,
    pub constellation: Constellation,
    pub carrier: Carrier,
}

/// [QcObservationsSamplingGapHistogram] regroups 
/// [QcSamplingHistogramAnalysis] results per [Constellation]
/// and [QcIndexing] providers.
pub struct QcObservationsSamplingGapHistogram {
    /// Histogram name/title
    pub name: String,

    /// Overall largest gap duration
    pub largest_gap: Duration,

    /// Overall shortest gap duration
    pub shortest_gap: Duration,

    /// Analysis
    pub results: HashMap<(QcIndexing, Constellation, Carrier), QcSamplingHistogramAnalysis>,
}

impl QcObservationsSamplingGapHistogram {

    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            results: Default::default(),
            largest_gap: Duration::ZERO,
            shortest_gap: Duration::ZERO,
        }
    }

    pub fn latch(&mut self, index: &QcIndexing, carrier: Carrier, gap: Duration) {
        let key = QcSamplingHistogramKey {
            index: indexing.clone(),
            constellation,
            carrier,
        };

        if let Some(results) = &mut self.results.get_mut(&key) {
            results.latch(gap);
        } else {
            let mut results =  QcSamplingHistogramAnalysis::default();
            results.latch(gap);
            self.results.insert(key, results);
        }
    }
}
