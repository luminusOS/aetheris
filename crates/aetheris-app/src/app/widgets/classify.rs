use super::*;

pub(crate) fn resource_icon_name(resource: &ResourceKind) -> &'static str {
    match resource.group.as_str() {
        "" => match resource.kind.as_str() {
            "Pod" => "lucide-box-symbolic",
            "ConfigMap" => "lucide-file-sliders-symbolic",
            "Secret" => "lucide-file-key-2-symbolic",
            "Namespace" => "lucide-orbit-symbolic",
            "Service" => "lucide-waypoints-symbolic",
            "Node" => "lucide-server-symbolic",
            "PersistentVolume" => "lucide-hard-drive-download-symbolic",
            "PersistentVolumeClaim" => "lucide-hard-drive-upload-symbolic",
            "Event" => "dialog-information-symbolic",
            "ServiceAccount" => "lucide-user-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "apps" => match resource.kind.as_str() {
            "ReplicaSet" => "lucide-layers-2-symbolic",
            "Deployment" => "lucide-layers-3-symbolic",
            "StatefulSet" => "lucide-database-symbolic",
            "DaemonSet" => "lucide-server-cog-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "batch" => match resource.kind.as_str() {
            "Job" => "lucide-cloud-cog-symbolic",
            "CronJob" => "lucide-timer-reset-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "networking.k8s.io" => match resource.kind.as_str() {
            "Ingress" => "lucide-radio-tower-symbolic",
            "IngressClass" => "lucide-cast-symbolic",
            "NetworkPolicy" => "lucide-globe-lock-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "events.k8s.io" => match resource.kind.as_str() {
            "Event" => "dialog-information-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "apiextensions.k8s.io" => match resource.kind.as_str() {
            "CustomResourceDefinition" => "lucide-toy-brick-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "storage.k8s.io" => match resource.kind.as_str() {
            "CSIDriver" => "lucide-warehouse-symbolic",
            "CSINode" => "lucide-cylinder-symbolic",
            "StorageClass" => "lucide-import-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "helm.toolkit.fluxcd.io" => match resource.kind.as_str() {
            "HelmRelease" => "lucide-package-open-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "source.toolkit.fluxcd.io" => match resource.kind.as_str() {
            "HelmChart" => "lucide-map-symbolic",
            "HelmRepository" => "lucide-library-symbolic",
            "GitRepository" => "lucide-folder-git-symbolic",
            "Bucket" => "lucide-paint-bucket-symbolic",
            "OCIRepository" => "lucide-container-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "monitoring.coreos.com" => match resource.kind.as_str() {
            "PodMonitor" => "lucide-package-search-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        _ => "lucide-blocks-symbolic",
    }
}

pub(crate) fn is_workload_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "Pod" | "Deployment" | "ReplicaSet" | "StatefulSet" | "DaemonSet" | "Job" | "CronJob"
    ) || matches!(resource.group.as_str(), "apps" | "batch")
}

pub(crate) fn is_network_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "Service" | "Ingress" | "EndpointSlice" | "NetworkPolicy" | "Endpoints"
    ) || matches!(
        resource.group.as_str(),
        "networking.k8s.io" | "discovery.k8s.io"
    )
}

pub(crate) fn is_storage_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "PersistentVolume"
            | "PersistentVolumeClaim"
            | "StorageClass"
            | "CSIDriver"
            | "CSINode"
            | "CSIStorageCapacity"
            | "VolumeAttachment"
    ) || resource.group == "storage.k8s.io"
}

pub(crate) fn is_configuration_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "ConfigMap"
            | "Secret"
            | "ResourceQuota"
            | "LimitRange"
            | "HorizontalPodAutoscaler"
            | "PodDisruptionBudget"
            | "PriorityClass"
    ) || matches!(
        resource.group.as_str(),
        "autoscaling" | "policy" | "scheduling.k8s.io"
    )
}

pub(crate) fn is_access_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "ServiceAccount"
            | "Role"
            | "RoleBinding"
            | "ClusterRole"
            | "ClusterRoleBinding"
            | "CertificateSigningRequest"
    ) || matches!(
        resource.group.as_str(),
        "rbac.authorization.k8s.io" | "certificates.k8s.io"
    )
}

pub(crate) fn is_cluster_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "Namespace"
            | "Node"
            | "ComponentStatus"
            | "RuntimeClass"
            | "CustomResourceDefinition"
            | "APIService"
            | "MutatingWebhookConfiguration"
            | "ValidatingWebhookConfiguration"
    )
}

pub(crate) fn available_icon_name<'a>(preferred: &'a str, fallback: &'a str) -> &'a str {
    let Some(display) = gtk::gdk::Display::default() else {
        return fallback;
    };
    let theme = gtk::IconTheme::for_display(&display);
    if theme.has_icon(preferred) {
        preferred
    } else {
        fallback
    }
}
