use super::*;

impl ResourceSection {
    pub(crate) const ALL: [Self; 7] = [
        Self::Workloads,
        Self::Network,
        Self::Storage,
        Self::Configuration,
        Self::Access,
        Self::Cluster,
        Self::Custom,
    ];

    pub(crate) fn label(self) -> String {
        match self {
            Self::Workloads => tr("Workloads"),
            Self::Network => tr("Network"),
            Self::Storage => tr("Storage"),
            Self::Configuration => tr("Configuration"),
            Self::Access => tr("Access"),
            Self::Cluster => tr("Cluster"),
            Self::Custom => tr("Custom"),
        }
    }

    pub(crate) fn icon_name(self) -> &'static str {
        match self {
            Self::Workloads => "grid-large-symbolic",
            Self::Network => "network-transmit-receive-symbolic",
            Self::Storage => "harddisk-symbolic",
            Self::Configuration => "rich-text-symbolic",
            Self::Access => "key-symbolic",
            Self::Cluster => "network-server-symbolic",
            Self::Custom => "puzzle-piece-symbolic",
        }
    }

    pub(crate) fn fallback_icon_name(self) -> &'static str {
        match self {
            Self::Workloads => "applications-system-symbolic",
            Self::Network => "network-workgroup-symbolic",
            Self::Storage => "drive-harddisk-symbolic",
            Self::Configuration => "preferences-system-symbolic",
            Self::Access => "changes-prevent-symbolic",
            Self::Cluster => "network-server-symbolic",
            Self::Custom => "application-x-addon-symbolic",
        }
    }

    pub(crate) fn for_resource(resource: &ResourceKind) -> Self {
        Self::ALL
            .iter()
            .copied()
            .find(|section| *section != Self::Custom && section.matches(resource))
            .unwrap_or(Self::Custom)
    }

    pub(crate) fn matches(self, resource: &ResourceKind) -> bool {
        match self {
            Self::Workloads => is_workload_resource(resource),
            Self::Network => is_network_resource(resource),
            Self::Storage => is_storage_resource(resource),
            Self::Configuration => is_configuration_resource(resource),
            Self::Access => is_access_resource(resource),
            Self::Cluster => is_cluster_resource(resource),
            Self::Custom => !Self::ALL
                .iter()
                .copied()
                .filter(|section| *section != Self::Custom)
                .any(|section| section.matches(resource)),
        }
    }
}
