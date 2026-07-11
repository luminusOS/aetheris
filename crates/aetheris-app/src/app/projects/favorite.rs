use super::*;

impl ObjectFavorite {
    pub(super) fn from_target(target: &DetailTarget) -> Self {
        Self {
            context: target.context.clone(),
            group: target.resource.group.clone(),
            version: target.resource.version.clone(),
            api_version: target.resource.api_version.clone(),
            kind: target.resource.kind.clone(),
            plural: target.resource.plural.clone(),
            namespace: target.namespace.clone(),
            name: target.name.clone(),
        }
    }

    pub(super) fn matches_target(&self, target: &DetailTarget) -> bool {
        self.context == target.context
            && self.group == target.resource.group
            && self.kind == target.resource.kind
            && self.namespace == target.namespace
            && self.name == target.name
    }

    pub(crate) fn resource(&self) -> ResourceKind {
        ResourceKind {
            group: self.group.clone(),
            version: self.version.clone(),
            api_version: self.api_version.clone(),
            kind: self.kind.clone(),
            plural: self.plural.clone(),
            scope: if self.namespace.is_some() {
                aetheris_kube::ResourceScope::Namespaced
            } else {
                aetheris_kube::ResourceScope::Cluster
            },
        }
    }

    pub(crate) fn namespace(&self) -> Option<String> {
        self.namespace.clone()
    }

    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }
}
