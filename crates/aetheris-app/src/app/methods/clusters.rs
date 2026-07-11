use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

impl App {
    pub(crate) fn ensure_cluster_summaries_loading(&mut self, sender: ComponentSender<Self>) {
        let pending: Vec<String> = self
            .visible_contexts()
            .iter()
            .map(|context| context.name.clone())
            .filter(|name| !self.cluster_summaries.contains_key(name))
            .collect();
        for context_name in pending {
            self.cluster_summaries
                .insert(context_name.clone(), ClusterSummaryState::Loading);
            sender.oneshot_command(async move { load_cluster_summary(context_name).await });
        }
    }

    pub(crate) fn refresh_cluster_summaries(&mut self, sender: ComponentSender<Self>) {
        let contexts = self
            .visible_contexts()
            .iter()
            .map(|context| context.name.clone())
            .collect::<Vec<_>>();
        for context_name in contexts {
            self.cluster_summaries
                .insert(context_name.clone(), ClusterSummaryState::Loading);
            sender.oneshot_command(async move { load_cluster_summary(context_name).await });
        }
        self.rebuild_cluster_list();
    }

    pub(crate) fn project_contexts(&self) -> Vec<&ContextInfo> {
        if self.projects.projects.is_empty() {
            return Vec::new();
        }

        let Some(project) = self.projects.selected_project() else {
            return self.contexts.iter().collect();
        };
        let allowed_contexts = project
            .contexts
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        self.contexts
            .iter()
            .filter(|context| allowed_contexts.contains(context.name.as_str()))
            .collect()
    }

    pub(crate) fn visible_contexts(&self) -> Vec<&ContextInfo> {
        self.project_contexts()
    }

    /// The add and edit flows share one cluster form; opening "add" must
    /// not show whatever the last add/edit left behind.
    pub(crate) fn reset_cluster_dialog_form(&self) {
        self.setup_name_entry.set_text("");
        self.setup_server_entry.set_text("");
        self.setup_token_entry.set_text("");
        self.setup_ca_entry.set_text("");
        self.setup_insecure_check.set_active(false);
    }

    pub(crate) fn set_cluster_dialog_editing(&mut self, editing: bool) {
        self.editing_cluster = editing;
        if editing {
            self.cluster_dialog.set_title(&tr("Edit Cluster"));
            self.cluster_token_title_label
                .set_label(&tr("Edit cluster"));
            self.cluster_token_back_button.set_visible(false);
            self.setup_button.set_label(&tr("Save"));
        } else {
            self.cluster_dialog.set_title(&tr("Add Cluster"));
            self.cluster_token_title_label
                .set_label(&tr("Connect with token"));
            self.cluster_token_back_button.set_visible(true);
            self.setup_button.set_label(&tr("Add Cluster"));
            self.editing_context_name = None;
        }
    }

    pub(crate) fn open_cluster_edit_dialog(
        &mut self,
        context_name: &str,
        server: &str,
        insecure_skip_tls_verify: bool,
        root: &<Self as Component>::Root,
    ) {
        self.setup_name_entry.set_text(context_name);
        self.setup_server_entry.set_text(server);
        self.setup_token_entry.set_text("");
        self.setup_ca_entry.set_text("");
        self.setup_insecure_check
            .set_active(insecure_skip_tls_verify);
        self.editing_context_name = Some(context_name.to_owned());
        self.set_cluster_dialog_editing(true);
        self.cluster_dialog_stack.set_visible_child_name("token");
        self.cluster_dialog.present(Some(root));
    }

    pub(crate) fn load_cluster(&mut self, sender: ComponentSender<Self>) {
        let Some(context) = self.selected_context.clone() else {
            self.loading = false;
            self.status = tr("Select a Kubernetes context.");
            self.sync_status();
            return;
        };

        self.show_object_list();
        self.stop_object_watch();
        self.stop_log_stream();
        self.stop_port_forward();
        self.object_cache.clear();
        self.object_cache_order.clear();
        self.detail.exec_target = None;
        self.detail.port_forward_target = None;
        self.loading = true;
        self.namespaces = with_all_namespace(Vec::new());
        self.resources.clear();
        self.objects.clear();
        self.selected_namespace = String::from("all");
        self.selected_resource = None;
        self.status = tr_format(
            "Discovering resources in {context}...",
            &[("{context}", context.clone())],
        );
        self.sync_dropdowns(Some(sender.clone()));
        self.rebuild_resource_list(Some(sender.clone()));
        self.rebuild_object_list();
        self.sync_terminal_controls();
        self.sync_port_forward_controls();
        self.sync_status();
        sender.oneshot_command(async move { load_cluster(context).await });
    }
}
