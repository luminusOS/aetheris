use super::super::utils::*;
use super::super::*;

pub(super) fn handle_objects_loaded_ok(
    app: &mut App,
    sender: ComponentSender<App>,
    token: u64,
    objects: Vec<ObjectSummary>,
) {
    if token != app.object_load_token {
        return;
    }
    app.loading = false;
    let count = objects.len();
    app.objects = objects;
    app.cache_current_objects();
    app.set_object_status(count);
    app.sync_status();
    app.rebuild_object_list();
    app.start_object_watch(sender);
}

pub(super) fn handle_objects_loaded_err(app: &mut App, token: u64, error: String) {
    if token != app.object_load_token {
        return;
    }
    app.loading = false;
    app.stop_object_watch();
    if let Some(objects) = app
        .current_object_cache_key()
        .and_then(|key| app.cached_objects(&key))
    {
        app.objects = objects;
    } else {
        app.objects.clear();
    }
    app.status = tr("Unable to list selected resource.");
    app.sync_status();
    app.rebuild_object_list();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_object_activated(app: &mut App, sender: ComponentSender<App>, index: i32) {
    let Some((context, resource, namespace, name)) = app.detail_request(index) else {
        return;
    };
    app.open_object_detail(context, resource, namespace, name, sender);
}

pub(super) fn handle_object_column_resized(
    app: &mut App,
    sender: &ComponentSender<App>,
    column: ObjectTableColumn,
    width: i32,
) {
    if app.projects.set_object_table_column_width(column, width) {
        app.schedule_project_save(sender);
    }
}

pub(super) fn handle_object_column_toggled(app: &mut App, index: u32) {
    let Some(column) = app.offerable_object_columns().get(index as usize).copied() else {
        return;
    };
    let visible = !app.projects.visible_object_columns.contains(&column);
    app.projects.set_object_column_visible(column, visible);
    app.save_projects_or_toast();
    app.sync_object_columns();
}

pub(super) fn handle_object_list_refresh_tick(app: &mut App) {
    app.flush_object_list_refresh();
}

pub(super) fn handle_object_watch_event(
    app: &mut App,
    sender: ComponentSender<App>,
    token: u64,
    event: ObjectWatchEvent,
) {
    if token != app.object_watch_token || app.loading {
        return;
    }
    match event {
        ObjectWatchEvent::Restarted(objects) => {
            app.objects = objects;
            app.cache_current_objects();
            app.schedule_object_list_refresh(&sender);
        }
        ObjectWatchEvent::Applied(object) => {
            app.upsert_object(object);
            app.cache_current_objects();
            app.schedule_object_list_refresh(&sender);
        }
        ObjectWatchEvent::Deleted(object) => {
            app.remove_object(&object);
            app.cache_current_objects();
            app.schedule_object_list_refresh(&sender);
        }
        ObjectWatchEvent::Error(error) => {
            app.status = tr_format("Live watch reconnecting: {error}", &[("{error}", error)]);
            app.sync_status();
        }
    }
}

pub(super) fn handle_object_watch_finished(app: &mut App, token: u64, result: Result<(), String>) {
    if token != app.object_watch_token {
        return;
    }
    app.object_watch_abort_handle = None;
    if let Err(error) = result {
        app.status = tr_format("Live watch stopped: {error}", &[("{error}", error)]);
        app.sync_status();
    }
}

pub(super) fn handle_search_changed(app: &mut App, query: String) {
    app.search_query = query;
    app.sync_status();
    app.rebuild_object_list();
}

pub(super) fn handle_status_filter_changed(app: &mut App, index: u32) {
    let Some(filter) = StatusFilter::ALL.get(index as usize).copied() else {
        return;
    };
    if app.selected_status_filters.contains(&filter) {
        app.selected_status_filters.remove(&filter);
    } else {
        app.selected_status_filters.insert(filter);
    }
    app.sync_status();
    app.sync_status_filter();
    app.rebuild_object_list();
}

pub(super) fn handle_favorite_object_activated(
    app: &mut App,
    sender: ComponentSender<App>,
    favorite: ObjectFavorite,
) {
    app.open_object_detail(
        favorite.context.clone(),
        favorite.resource(),
        favorite.namespace(),
        favorite.name.clone(),
        sender,
    );
}

pub(super) fn handle_toggle_current_object_favorite(app: &mut App, sender: ComponentSender<App>) {
    let Some(target) = app.detail.target.clone() else {
        return;
    };
    app.projects.toggle_object_favorite(&target);
    app.save_projects_or_toast();
    app.sync_detail_favorite_button();
    app.rebuild_favorite_object_list(Some(sender.clone()));
}

pub(super) fn handle_related_pod_activated(
    app: &mut App,
    sender: ComponentSender<App>,
    index: i32,
) {
    let Some(pod) = app.related_pod_at(index) else {
        return;
    };
    let Some(target) = app.detail.target.clone() else {
        return;
    };
    let namespace = Some(pod.namespace.clone());
    app.open_object_detail(
        target.context,
        pod_resource_kind(),
        namespace,
        pod.name,
        sender,
    );
}
