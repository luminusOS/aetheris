use super::super::ansi::*;
use super::super::object_detail::*;
use super::super::widgets::{available_icon_name, related_pods_column_view};
use super::super::yaml::*;
use super::super::*;

/// Builds every widget for the object detail pane (overview labels, YAML
/// editor, events/conditions/service/ingress tabs, related-Pods table, log
/// viewer, port-forward controls), wires every button/control to its
/// `AppMsg`, and assembles the final `DetailPane` model value. Returns the
/// pane alongside the rendered page `gtk::Box` (`ContentWidgets` needs the
/// latter; it isn't stored on `App` itself).
pub(super) fn build(sender: &ComponentSender<App>) -> (DetailPane, gtk::Box) {
    let detail_back_button = gtk::Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text(tr("Back to objects"))
        .visible(false)
        .build();
    detail_back_button.add_css_class("flat");
    let detail_name_label = detail_value_label();
    let detail_namespace_label = detail_value_label();
    let detail_status_label = detail_value_label();
    let detail_kind_label = detail_value_label();
    let detail_api_label = detail_value_label();
    let detail_age_label = detail_value_label();
    let detail_cpu_label = detail_value_label();
    let detail_memory_label = detail_value_label();
    let detail_container_metrics_list = gtk::ListBox::new();
    detail_container_metrics_list.add_css_class("boxed-list");
    detail_container_metrics_list.set_selection_mode(gtk::SelectionMode::None);
    let detail_scale_spin = gtk::SpinButton::with_range(0.0, 10000.0, 1.0);
    detail_scale_spin.set_numeric(true);
    detail_scale_spin.set_visible(false);
    let detail_scale_button = gtk::Button::builder()
        .label(tr("Scale"))
        .icon_name("view-grid-symbolic")
        .tooltip_text(tr("Scale deployment replicas"))
        .sensitive(false)
        .visible(false)
        .build();
    let detail_cordon_button = gtk::Button::builder()
        .label(tr("Cordon"))
        .icon_name("changes-prevent-symbolic")
        .tooltip_text(tr("Toggle node scheduling"))
        .sensitive(false)
        .visible(false)
        .build();
    let detail_drain_button = gtk::Button::builder()
        .label(tr("Drain"))
        .icon_name("edit-delete-symbolic")
        .tooltip_text(tr("Drain eligible pods from this node"))
        .sensitive(false)
        .visible(false)
        .build();
    detail_drain_button.add_css_class("destructive-action");
    let detail_explain_yaml_button = gtk::Button::builder()
        .label(tr("Explain"))
        .icon_name("dialog-information-symbolic")
        .tooltip_text(tr("Explain this YAML manifest"))
        .sensitive(false)
        .build();
    let detail_apply_button = gtk::Button::builder()
        .label(tr("Apply YAML"))
        .icon_name("document-send-symbolic")
        .tooltip_text(tr("Apply edited YAML to the cluster"))
        .sensitive(false)
        .build();
    detail_apply_button.add_css_class("suggested-action");
    let detail_download_yaml_button = gtk::Button::builder()
        .label(tr("Save YAML"))
        .icon_name("document-save-as-symbolic")
        .tooltip_text(tr("Save YAML to a local file"))
        .sensitive(false)
        .build();
    let detail_delete_button = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text(tr("Delete this object"))
        .sensitive(false)
        .visible(false)
        .build();
    detail_delete_button.add_css_class("destructive-action");
    detail_delete_button.add_css_class("flat");
    detail_delete_button.set_size_request(34, 34);
    detail_delete_button.set_valign(gtk::Align::Center);
    let detail_favorite_button = gtk::Button::builder()
        .icon_name(available_icon_name(
            "aetheris-object-favorite-outline-symbolic",
            "non-starred-symbolic",
        ))
        .tooltip_text(tr("Add to favorites"))
        .sensitive(false)
        .visible(false)
        .build();
    detail_favorite_button.add_css_class("flat");
    detail_favorite_button.set_size_request(34, 34);
    detail_favorite_button.set_valign(gtk::Align::Center);
    let detail_terminal_button = gtk::Button::builder()
        .icon_name("utilities-terminal-symbolic")
        .tooltip_text(tr("Open terminal"))
        .sensitive(false)
        .visible(false)
        .build();
    detail_terminal_button.add_css_class("flat");
    detail_terminal_button.set_size_request(34, 34);
    detail_terminal_button.set_valign(gtk::Align::Center);
    let detail_port_forward_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let detail_overview_section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let detail_expand_logs_button = gtk::Button::builder()
        .icon_name("view-fullscreen-symbolic")
        .tooltip_text(tr("Hide summary to see more of this tab"))
        .build();
    let detail_yaml_buffer = sourceview5::Buffer::new(None);
    let detail_yaml_error_label = gtk::Label::new(None);
    setup_yaml_buffer(&detail_yaml_buffer, &detail_yaml_error_label);
    let detail_events_list = gtk::ListBox::new();
    detail_events_list.add_css_class("boxed-list");
    detail_events_list.set_selection_mode(gtk::SelectionMode::None);
    let detail_conditions_list = gtk::ListBox::new();
    detail_conditions_list.add_css_class("boxed-list");
    detail_conditions_list.set_selection_mode(gtk::SelectionMode::None);
    let detail_service_ports_list = gtk::ListBox::new();
    detail_service_ports_list.add_css_class("boxed-list");
    detail_service_ports_list.set_selection_mode(gtk::SelectionMode::None);
    let detail_service_selectors_list = gtk::ListBox::new();
    detail_service_selectors_list.add_css_class("boxed-list");
    detail_service_selectors_list.set_selection_mode(gtk::SelectionMode::None);
    let detail_ingress_rules_list = gtk::ListBox::new();
    detail_ingress_rules_list.add_css_class("boxed-list");
    detail_ingress_rules_list.set_selection_mode(gtk::SelectionMode::None);
    let (detail_related_pods_view, detail_related_pods_store, detail_related_pods_sorted) =
        related_pods_column_view();
    let detail_related_pod_states = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .column_spacing(8)
        .row_spacing(8)
        .max_children_per_line(4)
        .min_children_per_line(1)
        .homogeneous(true)
        .hexpand(true)
        .build();
    detail_related_pod_states.add_css_class("deployment-pod-state-grid");
    let detail_related_pod_states_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let detail_related_pods_stack = gtk::Stack::builder()
        .hhomogeneous(false)
        .vhomogeneous(false)
        .build();
    let detail_related_pods_message = adw::StatusPage::builder()
        .title(tr("Pods are shown for Deployments"))
        .description(tr("Open a Deployment to inspect its related Pods."))
        .icon_name("dialog-information-symbolic")
        .build();
    detail_related_pods_message.add_css_class("compact");
    let no_containers = tr("No containers");
    let detail_log_container_dropdown = gtk::DropDown::from_strings(&[no_containers.as_str()]);
    detail_log_container_dropdown.set_sensitive(false);
    let detail_log_follow_check = gtk::CheckButton::builder().label(tr("Follow")).build();
    detail_log_follow_check.set_active(true);
    detail_log_follow_check.set_sensitive(false);
    let detail_log_timestamps_check = gtk::CheckButton::builder().label(tr("Timestamps")).build();
    detail_log_timestamps_check.set_active(false);
    detail_log_timestamps_check.set_sensitive(false);
    let detail_log_start_button = gtk::Button::builder()
        .label(tr("Start"))
        .icon_name("media-playback-start-symbolic")
        .tooltip_text(tr("Start log streaming"))
        .sensitive(false)
        .build();
    let detail_log_stop_button = gtk::Button::builder()
        .label(tr("Stop"))
        .icon_name("media-playback-stop-symbolic")
        .tooltip_text(tr("Stop log streaming"))
        .sensitive(false)
        .build();
    let detail_log_clear_button = gtk::Button::builder()
        .label(tr("Clear"))
        .icon_name("edit-clear-symbolic")
        .tooltip_text(tr("Clear visible logs"))
        .build();
    let detail_log_download_button = gtk::Button::builder()
        .icon_name("document-save-as-symbolic")
        .tooltip_text(tr("Save logs to a local file"))
        .build();
    let detail_log_status_label = gtk::Label::builder()
        .label(tr("Logs are available for Pods."))
        .xalign(0.0)
        .hexpand(true)
        .build();
    let detail_log_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    setup_log_highlighting(&detail_log_buffer);
    let detail_log_view = gtk::TextView::with_buffer(&detail_log_buffer);
    let detail_port_local_spin = gtk::SpinButton::with_range(0.0, 65535.0, 1.0);
    detail_port_local_spin.set_numeric(true);
    detail_port_local_spin.set_value(0.0);
    detail_port_local_spin.set_tooltip_text(Some(&tr("Local port, or 0 for an automatic port")));
    let detail_port_remote_spin = gtk::SpinButton::with_range(1.0, 65535.0, 1.0);
    detail_port_remote_spin.set_numeric(true);
    detail_port_remote_spin.set_value(8080.0);
    detail_port_remote_spin.set_tooltip_text(Some(&tr("Pod port to forward")));
    let detail_port_start_button = gtk::Button::builder()
        .label(tr("Start"))
        .icon_name("media-playback-start-symbolic")
        .tooltip_text(tr("Start port forwarding"))
        .sensitive(false)
        .build();
    let detail_port_stop_button = gtk::Button::builder()
        .label(tr("Stop"))
        .icon_name("media-playback-stop-symbolic")
        .tooltip_text(tr("Stop port forwarding"))
        .sensitive(false)
        .build();
    let detail_port_status_label = gtk::Label::builder()
        .label(tr("Port forwarding is available for Pods."))
        .xalign(0.0)
        .hexpand(true)
        .wrap(true)
        .build();
    let detail_stack = gtk::Stack::builder()
        .hhomogeneous(false)
        .vhomogeneous(false)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .build();
    let detail_page = build_object_detail_page(ObjectDetailWidgets {
        stack: &detail_stack,
        name: &detail_name_label,
        namespace: &detail_namespace_label,
        status: &detail_status_label,
        kind: &detail_kind_label,
        api_version: &detail_api_label,
        age: &detail_age_label,
        cpu: &detail_cpu_label,
        memory: &detail_memory_label,
        container_metrics_list: &detail_container_metrics_list,
        scale_spin: &detail_scale_spin,
        scale_button: &detail_scale_button,
        cordon_button: &detail_cordon_button,
        drain_button: &detail_drain_button,
        explain_yaml_button: &detail_explain_yaml_button,
        apply_button: &detail_apply_button,
        download_yaml_button: &detail_download_yaml_button,
        yaml_buffer: &detail_yaml_buffer,
        yaml_error_label: &detail_yaml_error_label,
        events_list: &detail_events_list,
        conditions_list: &detail_conditions_list,
        service_ports_list: &detail_service_ports_list,
        service_selectors_list: &detail_service_selectors_list,
        ingress_rules_list: &detail_ingress_rules_list,
        related_pods_view: &detail_related_pods_view,
        related_pod_states_section: &detail_related_pod_states_section,
        related_pod_states: &detail_related_pod_states,
        related_pods_stack: &detail_related_pods_stack,
        related_pods_message: &detail_related_pods_message,
        log_container_dropdown: &detail_log_container_dropdown,
        log_follow_check: &detail_log_follow_check,
        log_timestamps_check: &detail_log_timestamps_check,
        log_start_button: &detail_log_start_button,
        log_stop_button: &detail_log_stop_button,
        log_clear_button: &detail_log_clear_button,
        log_download_button: &detail_log_download_button,
        log_status_label: &detail_log_status_label,
        log_view: &detail_log_view,
        port_local_spin: &detail_port_local_spin,
        port_remote_spin: &detail_port_remote_spin,
        port_start_button: &detail_port_start_button,
        port_stop_button: &detail_port_stop_button,
        port_status_label: &detail_port_status_label,
        port_forward_group: &detail_port_forward_group,
        overview_section: &detail_overview_section,
        expand_logs_button: &detail_expand_logs_button,
    });

    detail_related_pods_view.connect_activate({
        let sender = sender.clone();
        move |_, position| sender.input(AppMsg::RelatedPodActivated(position as i32))
    });
    detail_back_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::BackToObjects)
    });
    detail_stack.connect_visible_child_name_notify({
        let sender = sender.clone();
        move |stack| {
            if let Some(name) = stack.visible_child_name() {
                sender.input(AppMsg::DetailTabChanged(name.to_string()));
            }
        }
    });
    detail_apply_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ApplyYaml)
    });
    detail_explain_yaml_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ExplainYaml)
    });
    detail_download_yaml_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DownloadYaml)
    });
    detail_delete_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DeleteObject)
    });
    detail_favorite_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ToggleCurrentObjectFavorite)
    });
    detail_terminal_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowPodTerminal)
    });
    detail_scale_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ScaleDeployment)
    });
    detail_cordon_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ToggleNodeScheduling)
    });
    detail_drain_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DrainNode)
    });
    detail_log_start_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StartPodLogs)
    });
    detail_log_stop_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StopPodLogs)
    });
    detail_log_clear_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ClearPodLogs)
    });
    detail_expand_logs_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ToggleDetailOverview)
    });
    detail_log_download_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DownloadLogs)
    });
    detail_port_start_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StartPodPortForward)
    });
    detail_port_stop_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StopPodPortForward)
    });

    let pane = DetailPane {
        back_button: detail_back_button,
        stack: detail_stack,
        name_label: detail_name_label,
        namespace_label: detail_namespace_label,
        status_label: detail_status_label,
        kind_label: detail_kind_label,
        api_label: detail_api_label,
        age_label: detail_age_label,
        cpu_label: detail_cpu_label,
        memory_label: detail_memory_label,
        container_metrics_list: detail_container_metrics_list,
        scale_spin: detail_scale_spin,
        scale_button: detail_scale_button,
        cordon_button: detail_cordon_button,
        drain_button: detail_drain_button,
        explain_yaml_button: detail_explain_yaml_button,
        apply_button: detail_apply_button,
        download_yaml_button: detail_download_yaml_button,
        delete_button: detail_delete_button,
        favorite_button: detail_favorite_button,
        terminal_button: detail_terminal_button,
        yaml_buffer: detail_yaml_buffer,
        events_list: detail_events_list,
        conditions_list: detail_conditions_list,
        service_ports_list: detail_service_ports_list,
        service_selectors_list: detail_service_selectors_list,
        ingress_rules_list: detail_ingress_rules_list,
        related_pods_store: detail_related_pods_store,
        related_pods_sorted: detail_related_pods_sorted,
        related_pod_states_section: detail_related_pod_states_section,
        related_pod_states: detail_related_pod_states,
        related_pods_stack: detail_related_pods_stack,
        related_pods_message: detail_related_pods_message,
        log_container_dropdown: detail_log_container_dropdown,
        log_follow_check: detail_log_follow_check,
        log_timestamps_check: detail_log_timestamps_check,
        log_start_button: detail_log_start_button,
        log_stop_button: detail_log_stop_button,
        log_status_label: detail_log_status_label,
        log_buffer: detail_log_buffer,
        log_view: detail_log_view,
        port_local_spin: detail_port_local_spin,
        port_remote_spin: detail_port_remote_spin,
        port_start_button: detail_port_start_button,
        port_stop_button: detail_port_stop_button,
        port_status_label: detail_port_status_label,
        port_forward_group: detail_port_forward_group,
        overview_section: detail_overview_section,
        expand_logs_button: detail_expand_logs_button,
        target: None,
        log_target: None,
        exec_target: None,
        port_forward_target: None,
        node_unschedulable: None,
        request_token: 0,
    };

    (pane, detail_page)
}
