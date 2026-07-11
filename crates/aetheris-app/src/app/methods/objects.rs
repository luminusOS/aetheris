use super::super::commands::*;
use super::super::*;
use super::object_cache::object_cache_key;

impl App {
    pub(crate) fn refresh_objects(&mut self, sender: ComponentSender<Self>) {
        let Some(context) = self.selected_context.clone() else {
            self.loading = false;
            self.status = tr("Select a Kubernetes context.");
            self.sync_status();
            return;
        };
        let Some(resource) = self.selected_resource_kind().cloned() else {
            self.loading = false;
            self.status = tr("Select a resource.");
            self.sync_status();
            return;
        };

        let namespace = if resource.is_namespaced() {
            Some(self.selected_namespace.clone())
        } else {
            None
        };
        self.stop_object_watch();
        self.object_load_token = self.object_load_token.saturating_add(1);
        let token = self.object_load_token;
        let cache_key = object_cache_key(&context, &resource, namespace.clone());
        self.loading = true;
        if let Some(objects) = self.cached_objects(&cache_key) {
            let count = objects.len();
            self.objects = objects;
            self.set_object_status(count);
            self.status = tr_format(
                "Refreshing {resource}...",
                &[("{resource}", resource.label())],
            );
        } else {
            self.objects.clear();
            self.status = tr_format("Loading {resource}...", &[("{resource}", resource.label())]);
        }
        self.sync_status();
        self.rebuild_object_list();
        sender.oneshot_command(
            async move { list_objects(token, context, resource, namespace).await },
        );
    }

    pub(crate) fn selected_resource_kind(&self) -> Option<&ResourceKind> {
        self.selected_resource
            .and_then(|index| self.resources.get(index))
    }

    pub(crate) fn set_object_status(&mut self, total: usize) {
        let resource = self
            .selected_resource_kind()
            .map(ResourceKind::label)
            .unwrap_or_else(|| tr("objects"));
        let filtered = self.filtered_objects().len();

        self.status = if self.search_query.trim().is_empty() {
            format!(
                "{} {} in {}",
                total,
                if total == 1 { "object" } else { "objects" },
                resource
            )
        } else {
            tr_format(
                "{filtered}/{total} objects in {resource}",
                &[
                    ("{filtered}", filtered.to_string()),
                    ("{total}", total.to_string()),
                    ("{resource}", resource.to_string()),
                ],
            )
        };
    }
}
