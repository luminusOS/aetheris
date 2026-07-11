use super::super::utils::*;
use super::super::*;

impl App {
    pub(crate) fn current_object_cache_key(&self) -> Option<ObjectCacheKey> {
        let context = self.selected_context.as_ref()?;
        let resource = self.selected_resource_kind()?;
        let namespace = resource
            .is_namespaced()
            .then(|| self.selected_namespace.clone());
        Some(object_cache_key(context, resource, namespace))
    }

    pub(crate) fn cached_objects(&mut self, key: &ObjectCacheKey) -> Option<Vec<ObjectSummary>> {
        let objects = self.object_cache.get(key)?.clone();
        self.touch_object_cache_key(key.clone());
        Some(objects)
    }

    pub(crate) fn cache_current_objects(&mut self) {
        if let Some(key) = self.current_object_cache_key() {
            self.cache_objects(key, self.objects.clone());
        }
    }

    pub(crate) fn cache_objects(&mut self, key: ObjectCacheKey, objects: Vec<ObjectSummary>) {
        self.object_cache.insert(key.clone(), objects);
        self.touch_object_cache_key(key);
        while self.object_cache_order.len() > OBJECT_CACHE_LIMIT {
            if let Some(oldest) = self.object_cache_order.pop_front() {
                self.object_cache.remove(&oldest);
            }
        }
    }

    pub(crate) fn touch_object_cache_key(&mut self, key: ObjectCacheKey) {
        self.object_cache_order.retain(|existing| existing != &key);
        self.object_cache_order.push_back(key);
    }

    pub(crate) fn clear_object_cache(&mut self) {
        self.object_cache.clear();
        self.object_cache_order.clear();
    }

    /// Merges one watch event into `objects` without sorting or repainting;
    /// both are deferred to `flush_object_list_refresh` so an event burst
    /// costs one refresh instead of thousands.
    pub(crate) fn upsert_object(&mut self, object: ObjectSummary) {
        if let Some(existing) = self
            .objects
            .iter_mut()
            .find(|existing| same_object(existing, &object))
        {
            *existing = object;
        } else {
            self.objects.push(object);
        }
    }

    pub(crate) fn remove_object(&mut self, object: &ObjectSummary) {
        self.objects
            .retain(|existing| !same_object(existing, object));
    }

    pub(crate) fn filtered_objects(&self) -> Vec<&ObjectSummary> {
        let query = self.search_query.trim().to_ascii_lowercase();
        self.objects
            .iter()
            .filter(|object| {
                StatusFilter::matches_any(&object.status, &self.selected_status_filters)
            })
            .filter(|object| query.is_empty() || object_matches(object, &query))
            .collect()
    }

    /// Schedules a coalesced object-list refresh. Any number of calls while
    /// one is pending collapse into a single `ObjectListRefreshTick`.
    pub(crate) fn schedule_object_list_refresh(&mut self, sender: &ComponentSender<Self>) {
        if self.object_list_refresh_scheduled {
            return;
        }
        self.object_list_refresh_scheduled = true;
        let sender = sender.clone();
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
            sender.input(AppMsg::ObjectListRefreshTick);
        });
    }

    pub(crate) fn flush_object_list_refresh(&mut self) {
        self.object_list_refresh_scheduled = false;
        if self.loading {
            return;
        }
        sort_objects(&mut self.objects);
        self.set_object_status(self.objects.len());
        self.sync_status();
        self.rebuild_object_list();
    }
}

pub(super) fn object_cache_key(
    context: &str,
    resource: &ResourceKind,
    namespace: Option<String>,
) -> ObjectCacheKey {
    ObjectCacheKey {
        context: context.to_owned(),
        group: resource.group.clone(),
        version: resource.version.clone(),
        kind: resource.kind.clone(),
        plural: resource.plural.clone(),
        namespace,
    }
}

fn same_object(left: &ObjectSummary, right: &ObjectSummary) -> bool {
    left.name == right.name && left.namespace == right.namespace
}

fn sort_objects(objects: &mut [ObjectSummary]) {
    objects.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
    });
}
