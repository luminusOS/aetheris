use super::ansi::*;
use super::commands::*;
use super::layout::*;
use super::object_detail::*;
use super::yaml::*;
use super::*;

mod clusters;
mod detail_signals;
mod namespaces;
mod object_table;
mod projects;
mod window_actions;
use clusters::ClustersWidgets;
use detail_signals::{DetailSignalWidgets, connect_detail_signals};
use namespaces::NamespacesWidgets;
use object_table::ObjectTableWidgets;
use projects::ProjectsWidgets;

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppMsg;

    view! {
        adw::ApplicationWindow {
            set_title: Some("Aetheris"),
            set_default_size: (1040, 720),

            #[local_ref]
            toaster -> adw::ToastOverlay {
                #[local_ref]
                root_stack -> gtk::Stack {}
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        super::style::load_app_css();

        let ProjectsWidgets {
            project_list,
            project_title_label,
            add_project_button,
            projects_empty_page,
            projects_content_stack,
            project_name_entry,
            project_create_button,
            project_dialog_description,
            project_menu_button,
            project_dialog,
            projects_home_button,
        } = projects::build(&sender);

        let ClustersWidgets {
            context_selector_label,
            cluster_back_button,
            cluster_menu_button,
            cluster_refresh_button,
            cluster_list,
            add_cluster_button,
            import_cluster_button,
            clusters_empty_page,
            clusters_content_stack,
            resource_list,
            favorite_object_list,
            cluster_dialog_stack,
            setup_name_entry,
            setup_server_entry,
            setup_token_entry,
            setup_ca_entry,
            setup_insecure_check,
            setup_button,
            cluster_token_title_label,
            cluster_token_back_button,
            cluster_dialog,
        } = clusters::build(&sender);
        let NamespacesWidgets {
            namespace_menu_button,
            namespace_selector_label,
            namespace_list,
            custom_namespace_entry,
            custom_namespace_button,
            custom_namespace_dialog,
            rename_namespace_entry,
            rename_namespace_button,
            rename_namespace_dialog,
        } = namespaces::build(&sender);
        let ObjectTableWidgets {
            search_entry,
            status_filter_list,
            column_filter_list,
            create_yaml_button,
            refresh_button,
            create_yaml_dialog,
            create_yaml_buffer,
            create_yaml_apply_button,
            object_store,
            object_view,
            object_sorted,
            object_columns,
            object_list_stack,
        } = object_table::build(&sender);
        let detail_back_button = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text(tr("Back to objects"))
            .visible(false)
            .build();
        detail_back_button.add_css_class("flat");
        let content_title_label = gtk::Label::new(Some(&tr("Objects")));
        content_title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        content_title_label.set_max_width_chars(24);
        let content_header_stack = gtk::Stack::new();
        content_header_stack.set_hhomogeneous(false);
        content_header_stack.set_vhomogeneous(false);
        let content_stack = gtk::Stack::builder()
            .hhomogeneous(false)
            .vhomogeneous(false)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        let status_label = gtk::Label::builder()
            .label(tr("Loading kubeconfig..."))
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let spinner = gtk::Spinner::builder().spinning(true).visible(true).build();
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
            .icon_name(super::widgets::available_icon_name(
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
            super::widgets::related_pods_column_view();
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
        let detail_log_timestamps_check =
            gtk::CheckButton::builder().label(tr("Timestamps")).build();
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
        detail_port_local_spin
            .set_tooltip_text(Some(&tr("Local port, or 0 for an automatic port")));
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

        let toaster = adw::ToastOverlay::new();
        let root_stack = gtk::Stack::new();
        root_stack.set_hhomogeneous(false);
        root_stack.set_vhomogeneous(false);
        // Nautilus-style responsive sidebar: an overlay split view keeps the
        // content pane always visible and slides the sidebar over it when
        // collapsed, toggled from a header button — instead of the
        // back-button navigation a NavigationSplitView would impose (which
        // also stacked a second, automatic back button next to our own on
        // the detail page whenever the view was collapsed).
        let split_view = adw::OverlaySplitView::builder()
            .min_sidebar_width(180.0)
            .max_sidebar_width(240.0)
            .sidebar_width_fraction(0.22)
            .collapsed(false)
            .enable_show_gesture(false)
            .enable_hide_gesture(false)
            .build();
        let sidebar_toggle_button = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(tr("Show Sidebar"))
            .visible(false)
            .build();
        sidebar_toggle_button.add_css_class("flat");
        split_view
            .bind_property("show-sidebar", &sidebar_toggle_button, "active")
            .bidirectional()
            .sync_create()
            .build();
        // Px, not Sp: on Windows GTK folds the display scale into
        // gtk-xft-dpi on top of the surface scale, so an Sp threshold gets
        // scaled twice and can exceed even a maximized window — leaving the
        // app permanently collapsed. Logical pixels are already
        // scale-corrected on every backend.
        let compact_layout = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            900.0,
            adw::LengthUnit::Px,
        ));
        compact_layout.add_setters(&[
            (&split_view, "collapsed", true),
            (&split_view, "enable-show-gesture", true),
            (&split_view, "enable-hide-gesture", true),
        ]);
        compact_layout.add_setter(&sidebar_toggle_button, "visible", Some(&true.to_value()));
        root.add_breakpoint(compact_layout);

        let sidebar = build_sidebar(SidebarWidgets {
            cluster_title: &context_selector_label,
            cluster_back_button: &cluster_back_button,
            cluster_menu_button: &cluster_menu_button,
            namespace_menu_button: &namespace_menu_button,
            resource_list: &resource_list,
            favorite_object_list: &favorite_object_list,
        });
        let content = build_content(ContentWidgets {
            sidebar_toggle_button: &sidebar_toggle_button,
            detail_back_button: &detail_back_button,
            delete_button: &detail_delete_button,
            favorite_button: &detail_favorite_button,
            create_yaml_button: &create_yaml_button,
            refresh_button: &refresh_button,
            terminal_button: &detail_terminal_button,
            search_entry: &search_entry,
            status_filter_list: &status_filter_list,
            column_filter_list: &column_filter_list,
            title: &content_title_label,
            header_stack: &content_header_stack,
            content_stack: &content_stack,
            status_label: &status_label,
            spinner: &spinner,
            object_view: &object_view,
            object_list_stack: &object_list_stack,
            detail_page: &detail_page,
        });
        split_view.set_sidebar(Some(&sidebar));
        split_view.set_content(Some(&content));
        root_stack.add_named(
            &build_projects_page(
                &project_list,
                &add_project_button,
                &projects_content_stack,
                &projects_empty_page,
            ),
            Some("projects"),
        );
        root_stack.add_named(
            &build_clusters_page(ClustersPageWidgets {
                projects_home_button: &projects_home_button,
                project_title: &project_title_label,
                project_menu_button: &project_menu_button,
                refresh_button: &cluster_refresh_button,
                cluster_list: &cluster_list,
                add_cluster_button: &add_cluster_button,
                import_cluster_button: &import_cluster_button,
                content_stack: &clusters_content_stack,
                empty_page: &clusters_empty_page,
            }),
            Some("clusters"),
        );
        root_stack.add_named(&split_view, Some("browser"));
        root_stack.set_visible_child_name("projects");

        window_actions::connect(&root, &sender);
        detail_related_pods_view.connect_activate({
            let sender = sender.clone();
            move |_, position| sender.input(AppMsg::RelatedPodActivated(position as i32))
        });
        connect_detail_signals(
            DetailSignalWidgets {
                back_button: &detail_back_button,
                stack: &detail_stack,
                apply_button: &detail_apply_button,
                explain_yaml_button: &detail_explain_yaml_button,
                download_yaml_button: &detail_download_yaml_button,
                delete_button: &detail_delete_button,
                favorite_button: &detail_favorite_button,
                terminal_button: &detail_terminal_button,
                scale_button: &detail_scale_button,
                cordon_button: &detail_cordon_button,
                drain_button: &detail_drain_button,
                log_start_button: &detail_log_start_button,
                log_stop_button: &detail_log_stop_button,
                log_clear_button: &detail_log_clear_button,
                expand_logs_button: &detail_expand_logs_button,
                log_download_button: &detail_log_download_button,
                port_start_button: &detail_port_start_button,
                port_stop_button: &detail_port_stop_button,
            },
            &sender,
        );

        let model = App {
            projects: ProjectStore::default(),
            contexts: Vec::new(),
            namespaces: vec![String::from("default")],
            resources: Vec::new(),
            objects: Vec::new(),
            object_list_refresh_scheduled: false,
            project_save_scheduled: false,
            selected_context: None,
            selected_namespace: String::from("default"),
            selected_resource_section: ResourceSection::Workloads,
            selected_resource: None,
            search_query: String::new(),
            selected_status_filters: StatusFilter::default_filters(),
            loading: true,
            status: tr("Loading kubeconfig..."),
            toaster,
            root_stack,
            split_view,
            project_list,
            project_title_label,
            add_project_button,
            projects_content_stack,
            cluster_back_button,
            cluster_menu_button,
            cluster_refresh_button,
            context_selector_label,
            cluster_list,
            add_cluster_button,
            import_cluster_button,
            clusters_content_stack,
            cluster_summaries: std::collections::HashMap::new(),
            namespace_menu_button,
            namespace_selector_label,
            namespace_list,
            search_entry,
            status_filter_list,
            column_filter_list,
            create_yaml_button,
            refresh_button,
            content_title_label,
            content_header_stack,
            content_stack,
            status_label,
            spinner,
            resource_list,
            favorite_object_list,
            object_store,
            object_sorted,
            object_columns,
            object_list_stack,
            object_cache: HashMap::new(),
            object_cache_order: VecDeque::new(),
            detail: DetailPane {
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
            },
            object_load_token: 0,
            object_watch_token: 0,
            object_watch_abort_handle: None,
            log_streaming: false,
            log_stream_token: 0,
            log_abort_handle: None,
            exec_token: 0,
            terminal_sessions: HashMap::new(),
            port_forwarding: false,
            port_forward_token: 0,
            port_forward_abort_handle: None,
            custom_namespace_dialog,
            custom_namespace_entry,
            custom_namespace_button,
            rename_namespace_dialog,
            rename_namespace_entry,
            rename_namespace_button,
            renaming_namespace: None,
            project_dialog,
            project_dialog_description,
            project_name_entry,
            project_create_button,
            editing_project_name: None,
            create_yaml_dialog,
            create_yaml_buffer,
            create_yaml_apply_button,
            cluster_dialog,
            cluster_dialog_stack,
            cluster_token_title_label,
            cluster_token_back_button,
            setup_name_entry,
            setup_server_entry,
            setup_token_entry,
            setup_ca_entry,
            setup_insecure_check,
            setup_button,
            editing_cluster: false,
            editing_context_name: None,
        };

        let toaster = model.toaster.clone();
        let root_stack = model.root_stack.clone();
        let widgets = view_output!();

        sender.oneshot_command(load_state());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.handle_msg(msg, sender, _root);
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.handle_msg(msg, sender, _root);
    }
}
