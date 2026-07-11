use super::*;

impl StatusFilter {
    pub(crate) const ALL: [Self; 6] = [
        Self::Ready,
        Self::Available,
        Self::Unavailable,
        Self::Running,
        Self::Pending,
        Self::Failed,
    ];

    pub(crate) fn label(self) -> String {
        match self {
            Self::Ready => tr("Ready"),
            Self::Available => tr("Available"),
            Self::Unavailable => tr("Unavailable"),
            Self::Running => tr("Running"),
            Self::Pending => tr("Pending"),
            Self::Failed => tr("Failed"),
        }
    }

    pub(crate) fn matches(self, status: &str) -> bool {
        status
            .split_whitespace()
            .next()
            .is_some_and(|part| part.eq_ignore_ascii_case(self.keyword()))
    }

    pub(crate) fn matches_any(status: &str, filters: &BTreeSet<Self>) -> bool {
        if filters.len() == Self::ALL.len() {
            return true;
        }
        filters.iter().any(|filter| filter.matches(status))
    }

    pub(crate) fn default_filters() -> BTreeSet<Self> {
        Self::ALL.into_iter().collect()
    }

    pub(crate) fn keyword(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Available => "Available",
            Self::Unavailable => "Unavailable",
            Self::Running => "Running",
            Self::Pending => "Pending",
            Self::Failed => "Failed",
        }
    }
}
