use super::*;

impl ObjectColumn {
    pub(crate) const ALL: [Self; 10] = [
        Self::Image,
        Self::Namespace,
        Self::Target,
        Self::Selector,
        Self::IngressClass,
        Self::Status,
        Self::Cpu,
        Self::Memory,
        Self::Api,
        Self::Age,
    ];

    pub(crate) fn label(self) -> String {
        match self {
            Self::Image => tr("Image"),
            Self::Namespace => tr("Namespace"),
            Self::Target => tr("Target"),
            Self::Selector => tr("Selector"),
            Self::IngressClass => tr("Ingress Class"),
            Self::Status => tr("Status"),
            Self::Cpu => tr("CPU"),
            Self::Memory => tr("Memory"),
            Self::Api => tr("API"),
            Self::Age => tr("Age"),
        }
    }

    pub(crate) fn default_width(self) -> i32 {
        match self {
            Self::Image => OBJECT_IMAGE_WIDTH,
            Self::Namespace => OBJECT_NAMESPACE_WIDTH,
            Self::Target => 156,
            Self::Selector => 180,
            Self::IngressClass => 144,
            Self::Status => OBJECT_STATUS_WIDTH,
            Self::Cpu | Self::Memory => OBJECT_METRIC_WIDTH,
            Self::Api => OBJECT_API_WIDTH,
            Self::Age => OBJECT_AGE_WIDTH,
        }
    }
}

pub(super) fn default_object_columns() -> Vec<ObjectColumn> {
    ObjectColumn::ALL.to_vec()
}
