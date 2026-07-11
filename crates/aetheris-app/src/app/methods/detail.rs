use super::super::commands::*;
use super::super::object_detail::*;
use super::super::utils::*;
use super::super::widgets::*;
use super::super::*;

impl App {
    pub(crate) fn detail_request(
        &self,
        index: i32,
    ) -> Option<(String, ResourceKind, Option<String>, String)> {
        let object = sorted_model_object(&self.object_sorted, index)?;
        let context = self.selected_context.clone()?;
        let resource = self.selected_resource_kind()?.clone();
        let namespace = resource.is_namespaced().then_some(object.namespace);

        Some((context, resource, namespace, object.name))
    }

    pub(crate) fn related_pod_at(&self, index: i32) -> Option<ObjectSummary> {
        sorted_model_object(&self.detail.related_pods_sorted, index)
    }

    pub(crate) fn open_object_detail(
        &mut self,
        context: String,
        resource: ResourceKind,
        namespace: Option<String>,
        name: String,
        sender: ComponentSender<Self>,
    ) {
        self.stop_log_stream();
        self.stop_port_forward();
        self.detail.log_buffer.set_text("");
        self.reset_detail_overview_layout();
        // Don't switch tabs yet: the previous object's detail page (and
        // whichever tab the user was on) stays on screen until the new
        // object's data actually arrives, so resetting here would flash
        // back to YAML on the OLD object before the new one loads.
        // `sync_detail_tabs` (run once new data is in) already falls back
        // to "yaml" if the current tab isn't valid for the new object.
        self.detail.target = Some(DetailTarget {
            context: context.clone(),
            resource: resource.clone(),
            namespace: namespace.clone(),
            name: name.clone(),
        });
        self.detail.log_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail.exec_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail.port_forward_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail.request_token = self.detail.request_token.saturating_add(1);
        let detail_token = self.detail.request_token;
        self.sync_log_controls();
        self.sync_terminal_controls();
        self.sync_port_forward_controls();

        self.loading = true;
        self.status = tr_format("Loading details for {name}...", &[("{name}", name.clone())]);
        self.sync_status();
        sender.oneshot_command(async move {
            load_object_detail(detail_token, context, resource, namespace, name).await
        });
    }

    pub(crate) fn reset_detail_overview_layout(&self) {
        self.detail.overview_section.set_visible(true);
        self.detail
            .expand_logs_button
            .set_icon_name("view-fullscreen-symbolic");
        self.detail
            .expand_logs_button
            .set_tooltip_text(Some(&tr("Hide summary to see more of this tab")));
    }

    pub(crate) fn populate_detail_dialog(&mut self, detail: &ObjectDetail) {
        self.detail.name_label.set_label(&detail.name);
        self.detail.namespace_label.set_label(&detail.namespace);
        self.detail.status_label.set_label(&detail.status);
        self.detail.kind_label.set_label(&detail.kind);
        self.detail.api_label.set_label(&detail.api_version);
        self.detail.age_label.set_label(&detail.age);
        self.detail.cpu_label.set_label(
            detail
                .metrics
                .as_ref()
                .map(|usage| usage.cpu.as_str())
                .unwrap_or("-"),
        );
        self.detail.memory_label.set_label(
            detail
                .metrics
                .as_ref()
                .map(|usage| usage.memory.as_str())
                .unwrap_or("-"),
        );
        self.detail.yaml_buffer.set_text(&detail.yaml);
        self.sync_detail_favorite_button();
        self.detail.node_unschedulable = detail.node_unschedulable;
        self.detail
            .scale_spin
            .set_value(detail.replicas.unwrap_or_default().into());
        self.detail
            .scale_spin
            .set_visible(detail.replicas.is_some());
        self.detail
            .scale_button
            .set_visible(detail.replicas.is_some());
        self.detail
            .cordon_button
            .set_visible(detail.node_unschedulable.is_some());
        self.detail
            .drain_button
            .set_visible(detail.node_unschedulable.is_some());
        self.detail.explain_yaml_button.set_sensitive(true);
        if let Some(unschedulable) = detail.node_unschedulable {
            let label = if unschedulable {
                tr("Uncordon")
            } else {
                tr("Cordon")
            };
            self.detail.cordon_button.set_label(&label);
        }
        rebuild_detail_events(&self.detail.events_list, detail);
        rebuild_detail_conditions(&self.detail.conditions_list, detail);
        rebuild_service_ports(&self.detail.service_ports_list, detail);
        rebuild_service_selectors(&self.detail.service_selectors_list, detail);
        rebuild_ingress_rules(&self.detail.ingress_rules_list, detail);
        rebuild_related_pods(
            &self.detail.related_pods_store,
            &self.detail.related_pods_stack,
            &self.detail.related_pods_message,
            detail,
        );
        rebuild_related_pod_states(
            &self.detail.related_pod_states_section,
            &self.detail.related_pod_states,
            detail,
        );
        rebuild_container_metrics(&self.detail.container_metrics_list, detail);
        self.sync_detail_tabs(detail);
        self.sync_terminal_controls();
        self.sync_port_forward_controls();
    }

    pub(crate) fn sync_detail_tabs(&self, detail: &ObjectDetail) {
        let show_logs = detail.kind == "Pod" && !detail.containers.is_empty();
        self.detail
            .port_forward_group
            .set_visible(detail.kind == "Pod");
        let show_pods = detail.kind == "Deployment";
        let show_conditions = !detail.conditions.is_empty();
        let show_containers = detail.kind == "Pod";
        let show_service_tabs = detail.kind == "Service";
        let show_ingress_rules = detail.kind == "Ingress";

        set_stack_page(
            &self.detail.stack,
            "pods",
            show_pods,
            &tr_format(
                "Pods ({count})",
                &[("{count}", detail.related_pods.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "conditions",
            show_conditions,
            &tr_format(
                "Conditions ({count})",
                &[("{count}", detail.conditions.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "containers",
            show_containers,
            &tr_format(
                "Containers ({count})",
                &[("{count}", detail.containers.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "service-ports",
            show_service_tabs,
            &tr_format(
                "Ports ({count})",
                &[("{count}", detail.service_ports.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "service-selectors",
            show_service_tabs,
            &tr_format(
                "Selectors ({count})",
                &[("{count}", detail.service_selectors.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "ingress-rules",
            show_ingress_rules,
            &tr_format(
                "Rules ({count})",
                &[("{count}", detail.ingress_rules.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "events",
            true,
            &tr_format(
                "Recent Events ({count})",
                &[("{count}", detail.events.len().to_string())],
            ),
        );
        set_stack_page(&self.detail.stack, "logs", show_logs, &tr("Logs"));
        set_stack_page(&self.detail.stack, "yaml", true, &tr("YAML"));

        let visible_name = self.detail.stack.visible_child_name();
        let visible_child_is_hidden = visible_name
            .as_deref()
            .and_then(|name| self.detail.stack.child_by_name(name))
            .is_some_and(|child| !child.is_visible());
        if visible_child_is_hidden {
            self.detail.stack.set_visible_child_name("yaml");
        }
    }

    pub(crate) fn sync_detail_favorite_button(&self) {
        let favorited = self
            .detail
            .target
            .as_ref()
            .is_some_and(|target| self.projects.is_object_favorite(target));
        self.detail
            .favorite_button
            .set_icon_name(available_icon_name(
                if favorited {
                    "aetheris-object-favorite-symbolic"
                } else {
                    "aetheris-object-favorite-outline-symbolic"
                },
                if favorited {
                    "starred-symbolic"
                } else {
                    "non-starred-symbolic"
                },
            ));
        let tooltip = if favorited {
            tr("Remove from favorites")
        } else {
            tr("Add to favorites")
        };
        self.detail.favorite_button.set_tooltip_text(Some(&tooltip));
    }
}

/// The object at a view position, resolved against the sorted model the
/// `ColumnView` actually displays (positions differ from the backing store
/// whenever a header-click sort is active).
fn sorted_model_object(model: &gtk::SortListModel, index: i32) -> Option<ObjectSummary> {
    let item = model
        .item(u32::try_from(index).ok()?)
        .and_downcast::<gtk::glib::BoxedAnyObject>()?;
    let object = item.borrow::<ObjectSummary>().clone();
    Some(object)
}
