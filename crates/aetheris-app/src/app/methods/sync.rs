use super::super::utils::*;
use super::super::widgets::*;
use super::super::*;

impl App {
    pub(crate) fn sync_object_columns(&self) {
        rebuild_column_filter_list(
            &self.column_filter_list,
            &self.offerable_object_columns(),
            &self.projects.visible_object_columns,
        );
        let offerable = self.offerable_object_columns();
        for (table_column, view_column) in &self.object_columns {
            match table_column {
                ObjectTableColumn::Name => {
                    view_column.set_fixed_width(self.projects.object_name_width());
                }
                ObjectTableColumn::Data(column) => {
                    view_column.set_visible(
                        offerable.contains(column)
                            && self.projects.visible_object_columns.contains(column),
                    );
                    view_column.set_fixed_width(self.projects.object_column_width(*column));
                }
            }
        }
    }

    pub(crate) fn offerable_object_columns(&self) -> Vec<ObjectColumn> {
        offerable_columns_for(self.selected_resource_kind())
    }

    pub(crate) fn sync_status_filter(&self) {
        rebuild_status_filter_list(&self.status_filter_list, &self.selected_status_filters);
    }

    pub(crate) fn sync_dropdowns(&self, sender: Option<ComponentSender<Self>>) {
        self.project_title_label
            .set_label(self.projects.selected_project_name());
        self.rebuild_project_list();

        self.rebuild_cluster_list();
        let context_label = self
            .selected_context
            .clone()
            .unwrap_or_else(|| tr("No cluster"));
        self.context_selector_label.set_label(&context_label);
        self.context_selector_label
            .set_tooltip_text(Some(&context_label));

        let namespace_choices = self.namespace_choices();
        let custom_namespaces: std::collections::HashSet<String> = self
            .projects
            .selected_project()
            .map(|project| {
                project
                    .custom_namespaces_for_context(self.selected_context.as_deref())
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default();
        while let Some(child) = self.namespace_list.first_child() {
            self.namespace_list.remove(&child);
        }
        for namespace in &namespace_choices {
            self.namespace_list.append(&namespace_selector_row(
                namespace,
                namespace == &self.selected_namespace,
                custom_namespaces.contains(namespace),
                sender.clone(),
            ));
        }
        self.namespace_list.append(&add_namespace_selector_row());
        let namespace_label = if self.selected_namespace.is_empty() {
            "default"
        } else {
            self.selected_namespace.as_str()
        };
        self.namespace_selector_label.set_label(namespace_label);
        self.namespace_selector_label
            .set_tooltip_text(Some(namespace_label));

        rebuild_status_filter_list(&self.status_filter_list, &self.selected_status_filters);
        self.sync_object_columns();
    }

    pub(crate) fn sync_status(&self) {
        self.status_label.set_label(&self.status);
        self.spinner.set_spinning(self.loading);
        self.spinner.set_visible(self.loading);
        self.refresh_button
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.create_yaml_button.set_sensitive(
            self.selected_context.is_some()
                && self.selected_resource_kind().is_some()
                && !self.loading,
        );
        self.search_entry
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.status_filter_list
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.column_filter_list
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.cluster_back_button.set_sensitive(true);
        self.cluster_menu_button
            .set_sensitive(self.selected_context.is_some());
        self.cluster_refresh_button
            .set_sensitive(!self.loading && !self.visible_contexts().is_empty());
        self.add_cluster_button.set_sensitive(!self.loading);
        self.import_cluster_button.set_sensitive(!self.loading);
        self.add_project_button.set_sensitive(!self.loading);
        self.namespace_menu_button.set_sensitive(
            self.selected_context.is_some()
                && !self.loading
                && self
                    .selected_resource_kind()
                    .is_none_or(ResourceKind::is_namespaced),
        );
        self.custom_namespace_button.set_sensitive(!self.loading);
        self.rename_namespace_button.set_sensitive(!self.loading);
        self.project_create_button.set_sensitive(!self.loading);
        self.detail
            .apply_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .download_yaml_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .explain_yaml_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .delete_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .favorite_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail.terminal_button.set_sensitive(
            self.detail
                .exec_target
                .as_ref()
                .is_some_and(|target| !target.containers.is_empty())
                && !self.loading,
        );
        self.detail.scale_button.set_sensitive(
            self.detail
                .target
                .as_ref()
                .is_some_and(|target| is_deployment_resource(&target.resource))
                && !self.loading,
        );
        self.detail.cordon_button.set_sensitive(
            self.detail
                .target
                .as_ref()
                .is_some_and(|target| is_node_resource(&target.resource))
                && !self.loading,
        );
        self.detail.drain_button.set_sensitive(
            self.detail
                .target
                .as_ref()
                .is_some_and(|target| is_node_resource(&target.resource))
                && !self.loading,
        );
        self.create_yaml_apply_button.set_sensitive(!self.loading);
    }

    pub(crate) fn grouped_resources(&self) -> Vec<(ResourceSection, Vec<(usize, &ResourceKind)>)> {
        ResourceSection::ALL
            .iter()
            .copied()
            .filter_map(|section| {
                let resources = self
                    .resources
                    .iter()
                    .enumerate()
                    .filter(|(_index, resource)| section.matches(resource))
                    .collect::<Vec<_>>();
                (!resources.is_empty()).then_some((section, resources))
            })
            .collect()
    }

    pub(crate) fn rebuild_project_list(&self) {
        rebuild_project_list(&self.project_list, &self.projects);
        self.projects_content_stack
            .set_visible_child_name(if self.projects.projects.is_empty() {
                "empty"
            } else {
                "content"
            });
    }

    pub(crate) fn rebuild_cluster_list(&self) {
        let visible_contexts = self.visible_contexts();
        rebuild_cluster_list(
            &self.cluster_list,
            &visible_contexts,
            &self.cluster_summaries,
            self.selected_context.as_deref(),
        );
        self.clusters_content_stack
            .set_visible_child_name(if visible_contexts.is_empty() {
                "empty"
            } else {
                "content"
            });
    }

    pub(crate) fn rebuild_resource_list(&self, sender: Option<ComponentSender<Self>>) {
        while let Some(child) = self.resource_list.first_child() {
            self.resource_list.remove(&child);
        }

        let resource_groups = self.grouped_resources();

        if resource_groups.is_empty() {
            let row = adw::ActionRow::builder()
                .title(tr("No resources"))
                .subtitle(tr("Connect to a cluster to load API resources."))
                .build();
            self.resource_list.append(&row);
            self.rebuild_favorite_object_list(sender);
            return;
        }

        for (section, resources) in resource_groups {
            let row = adw::ExpanderRow::builder()
                .title(section.label())
                .subtitle(resource_count_label(resources.len()))
                .expanded(section == self.selected_resource_section)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name(available_icon_name(
                section.icon_name(),
                section.fallback_icon_name(),
            )));

            for (resource_index, resource) in resources {
                let child = resource_row(resource, self.selected_resource == Some(resource_index));
                connect_resource_row(&child, sender.clone(), resource_index, section);
                row.add_row(&child);
            }

            self.resource_list.append(&row);
        }
        self.rebuild_favorite_object_list(sender);
    }

    pub(crate) fn rebuild_favorite_object_list(&self, sender: Option<ComponentSender<Self>>) {
        while let Some(child) = self.favorite_object_list.first_child() {
            self.favorite_object_list.remove(&child);
        }

        let Some(context) = self.selected_context.as_deref() else {
            let row = adw::ActionRow::builder()
                .title(tr("No favorites"))
                .subtitle(tr("Select a cluster to show favorite objects."))
                .build();
            self.favorite_object_list.append(&row);
            return;
        };
        let favorites = self.projects.favorite_objects_for_context(context);

        if favorites.is_empty() {
            let row = adw::ActionRow::builder()
                .title(tr("No favorites"))
                .subtitle(tr("Open an object and star it to keep it here."))
                .build();
            self.favorite_object_list.append(&row);
            return;
        }

        for favorite in favorites {
            let row = favorite_object_row(&favorite);
            connect_favorite_object_row(&row, sender.clone(), favorite);
            self.favorite_object_list.append(&row);
        }
    }

    /// Replaces the object table's backing model with the current filtered
    /// objects. The `ColumnView` is virtualized, so this is O(model) data
    /// work with only the on-screen row widgets ever being (re)built —
    /// tens of thousands of objects stay cheap.
    pub(crate) fn rebuild_object_list(&mut self) {
        let items: Vec<gtk::glib::BoxedAnyObject> = self
            .filtered_objects()
            .into_iter()
            .map(boxed_object)
            .collect();

        if items.is_empty() {
            self.object_store.remove_all();
            self.object_list_stack.set_visible_child_name("empty");
            return;
        }

        self.object_list_stack.set_visible_child_name("table");
        // One splice = one items-changed signal, instead of one per row.
        self.object_store
            .splice(0, self.object_store.n_items(), &items);
    }
}
