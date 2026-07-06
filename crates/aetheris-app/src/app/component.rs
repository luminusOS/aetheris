use super::ansi::*;
use super::commands::*;
use super::dialogs::*;
use super::layout::*;
use super::object_detail::*;
use super::widgets::{
    rebuild_column_filter_list, rebuild_status_filter_list, selector_button_child, selector_popover,
};
use super::yaml::*;
use super::*;

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
        load_app_css();

        let project_list = gtk::ListBox::new();
        project_list.set_hexpand(true);
        project_list.add_css_class("boxed-list");
        project_list.set_selection_mode(gtk::SelectionMode::None);
        let project_title_label = gtk::Label::new(Some(DEFAULT_PROJECT_NAME));
        project_title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        project_title_label.set_max_width_chars(22);
        let add_project_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add project")
            .build();
        add_project_button.add_css_class("flat");
        let projects_empty_add_button = gtk::Button::builder()
            .child(
                &adw::ButtonContent::builder()
                    .icon_name("list-add-symbolic")
                    .label("Add Project")
                    .build(),
            )
            .halign(gtk::Align::Center)
            .build();
        projects_empty_add_button.add_css_class("suggested-action");
        let projects_empty_page = adw::StatusPage::builder()
            .icon_name("folder-symbolic")
            .title("No Projects Yet")
            .description("Create a project to organize your clusters.")
            .valign(gtk::Align::Center)
            .vexpand(true)
            .build();
        projects_empty_page.set_child(Some(&projects_empty_add_button));
        let projects_content_stack = gtk::Stack::new();
        let context_selector_label = gtk::Label::new(Some("No cluster"));
        context_selector_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        context_selector_label.set_max_width_chars(22);
        let cluster_back_button = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text("Back to clusters")
            .build();
        cluster_back_button.add_css_class("flat");
        // HIG-style menu (as in Nautilus): a gio::Menu model instead of
        // custom buttons — no icons, and registered accelerators render on
        // the right edge of each item.
        let cluster_menu = gtk::gio::Menu::new();
        cluster_menu.append(Some("Edit Cluster…"), Some("win.cluster-edit"));
        cluster_menu.append(Some("Remove from Project"), Some("win.cluster-remove"));
        let cluster_menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Cluster options")
            .menu_model(&cluster_menu)
            .build();
        let cluster_refresh_button = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh cluster health")
            .build();
        cluster_refresh_button.add_css_class("flat");
        let cluster_list = gtk::ListBox::new();
        cluster_list.set_hexpand(true);
        cluster_list.add_css_class("boxed-list");
        cluster_list.set_selection_mode(gtk::SelectionMode::None);
        let add_cluster_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add cluster")
            .build();
        let import_cluster_button = gtk::Button::builder().label("Import").build();
        let clusters_empty_add_button = gtk::Button::builder()
            .child(
                &adw::ButtonContent::builder()
                    .icon_name("list-add-symbolic")
                    .label("Add Cluster")
                    .build(),
            )
            .halign(gtk::Align::Center)
            .build();
        clusters_empty_add_button.add_css_class("suggested-action");
        let clusters_empty_page = adw::StatusPage::builder()
            .icon_name("network-server-symbolic")
            .title("No Clusters Yet")
            .description("Add a cluster to start browsing this project.")
            .valign(gtk::Align::Center)
            .vexpand(true)
            .build();
        clusters_empty_page.set_child(Some(&clusters_empty_add_button));
        let clusters_content_stack = gtk::Stack::new();
        let namespace_selector_label = gtk::Label::new(Some("default"));
        let namespace_menu_button = gtk::MenuButton::new();
        namespace_menu_button.set_size_request(170, -1);
        namespace_menu_button.set_child(Some(&selector_button_child(&namespace_selector_label)));
        let namespace_list = gtk::ListBox::new();
        namespace_menu_button.set_popover(Some(&selector_popover(&namespace_list)));
        let custom_namespace_entry = adw::EntryRow::builder()
            .title("Namespace")
            .hexpand(true)
            .build();
        let custom_namespace_button = gtk::Button::builder()
            .label("Use")
            .tooltip_text("Use and save this namespace")
            .build();
        custom_namespace_button.add_css_class("suggested-action");
        let rename_namespace_entry = adw::EntryRow::builder()
            .title("Namespace")
            .hexpand(true)
            .build();
        let rename_namespace_button = gtk::Button::builder().label("Rename").build();
        rename_namespace_button.add_css_class("suggested-action");
        let project_name_entry = adw::EntryRow::builder()
            .title("Project Name")
            .hexpand(true)
            .build();
        let project_create_button = gtk::Button::builder().label("Create").build();
        project_create_button.add_css_class("suggested-action");
        let project_dialog_description =
            gtk::Label::new(Some("Separate clusters by environment or company"));
        let project_menu = gtk::gio::Menu::new();
        project_menu.append(Some("Rename Project…"), Some("win.project-rename"));
        project_menu.append(Some("Duplicate Project"), Some("win.project-duplicate"));
        project_menu.append(Some("Delete Project…"), Some("win.project-delete"));
        let project_menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Project options")
            .menu_model(&project_menu)
            .build();
        // width-chars is the entry's *minimum* width and the header bar
        // passes it straight up to the window: at 28 chars the whole content
        // pane bottomed out around 589px and the window refused to shrink
        // further (Adwaita then warns the toast overlay exceeds the window).
        // Keep the minimum tiny and let hexpand grow it into whatever the
        // header actually has.
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search")
            .width_chars(8)
            .max_width_chars(75)
            .build();
        let status_filter_list = gtk::FlowBox::new();
        status_filter_list.set_selection_mode(gtk::SelectionMode::None);
        status_filter_list.set_activate_on_single_click(true);
        status_filter_list.set_min_children_per_line(2);
        status_filter_list.set_max_children_per_line(3);
        status_filter_list.set_column_spacing(8);
        status_filter_list.set_row_spacing(8);
        rebuild_status_filter_list(&status_filter_list, &StatusFilter::default_filters());
        let default_columns = ProjectStore::default().visible_object_columns;
        let column_filter_list = gtk::FlowBox::new();
        column_filter_list.set_selection_mode(gtk::SelectionMode::None);
        column_filter_list.set_activate_on_single_click(true);
        column_filter_list.set_min_children_per_line(2);
        column_filter_list.set_max_children_per_line(3);
        column_filter_list.set_column_spacing(8);
        column_filter_list.set_row_spacing(8);
        rebuild_column_filter_list(&column_filter_list, &ObjectColumn::ALL, &default_columns);
        let projects_home_button = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text("Back to projects")
            .build();
        projects_home_button.add_css_class("flat");
        let create_yaml_button = gtk::Button::builder()
            .label("Create")
            .icon_name("document-new-symbolic")
            .tooltip_text("Create object from YAML")
            .sensitive(false)
            .build();
        let refresh_button = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh resources")
            .sensitive(false)
            .build();
        let detail_back_button = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text("Back to objects")
            .visible(false)
            .build();
        detail_back_button.add_css_class("flat");
        let content_title_label = gtk::Label::new(Some("Objects"));
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
            .label("Loading kubeconfig...")
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let spinner = gtk::Spinner::builder().spinning(true).visible(true).build();
        let resource_list = gtk::ListBox::new();
        resource_list.add_css_class("boxed-list");
        resource_list.set_selection_mode(gtk::SelectionMode::None);
        let object_store = gtk::gio::ListStore::new::<gtk::glib::BoxedAnyObject>();
        let object_view = gtk::ColumnView::builder()
            .single_click_activate(true)
            .reorderable(false)
            .build();
        object_view.add_css_class("aetheris-table");
        object_view.set_vexpand(true);

        let mut object_columns: Vec<(ObjectTableColumn, gtk::ColumnViewColumn)> = Vec::new();
        let name_column = gtk::ColumnViewColumn::new(
            Some("Name"),
            Some(super::widgets::object_name_column_factory()),
        );
        name_column.set_resizable(true);
        name_column.set_fixed_width(OBJECT_NAME_WIDTH);
        object_view.append_column(&name_column);
        object_columns.push((ObjectTableColumn::Name, name_column));
        for column in ObjectColumn::ALL {
            let view_column = gtk::ColumnViewColumn::new(
                Some(column.label()),
                Some(super::widgets::object_data_column_factory(column)),
            );
            view_column.set_resizable(true);
            view_column.set_fixed_width(column.default_width());
            view_column.set_sorter(super::widgets::object_column_sorter(column).as_ref());
            object_view.append_column(&view_column);
            object_columns.push((ObjectTableColumn::Data(column), view_column));
        }
        super::widgets::append_filler_column(&object_view);
        // Header-click sorting reorders the view, not the store, so
        // activation positions must be resolved against this sorted model.
        let object_sorted =
            gtk::SortListModel::new(Some(object_store.clone()), object_view.sorter());
        object_view.set_model(Some(&gtk::NoSelection::new(Some(object_sorted.clone()))));
        super::widgets::connect_sorted_header_highlight(&object_view);

        let object_list_stack = gtk::Stack::builder()
            .hhomogeneous(false)
            .vhomogeneous(false)
            .build();
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
            .label("Scale")
            .icon_name("view-grid-symbolic")
            .tooltip_text("Scale deployment replicas")
            .sensitive(false)
            .visible(false)
            .build();
        let detail_cordon_button = gtk::Button::builder()
            .label("Cordon")
            .icon_name("changes-prevent-symbolic")
            .tooltip_text("Toggle node scheduling")
            .sensitive(false)
            .visible(false)
            .build();
        let detail_drain_button = gtk::Button::builder()
            .label("Drain")
            .icon_name("edit-delete-symbolic")
            .tooltip_text("Drain eligible pods from this node")
            .sensitive(false)
            .visible(false)
            .build();
        detail_drain_button.add_css_class("destructive-action");
        let detail_explain_yaml_button = gtk::Button::builder()
            .label("Explain")
            .icon_name("dialog-information-symbolic")
            .tooltip_text("Explain this YAML manifest")
            .sensitive(false)
            .build();
        let detail_apply_button = gtk::Button::builder()
            .label("Apply YAML")
            .icon_name("document-send-symbolic")
            .tooltip_text("Apply edited YAML to the cluster")
            .sensitive(false)
            .build();
        detail_apply_button.add_css_class("suggested-action");
        let detail_download_yaml_button = gtk::Button::builder()
            .label("Save YAML")
            .icon_name("document-save-as-symbolic")
            .tooltip_text("Save YAML to a local file")
            .sensitive(false)
            .build();
        let detail_delete_button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text("Delete this object")
            .sensitive(false)
            .visible(false)
            .build();
        detail_delete_button.add_css_class("destructive-action");
        detail_delete_button.add_css_class("flat");
        detail_delete_button.set_size_request(34, 34);
        detail_delete_button.set_valign(gtk::Align::Center);
        let detail_terminal_button = gtk::Button::builder()
            .icon_name("utilities-terminal-symbolic")
            .tooltip_text("Open terminal")
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
            .tooltip_text("Hide summary to see more of this tab")
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
        let (detail_related_pods_view, detail_related_pods_store, detail_related_pods_sorted) =
            super::widgets::related_pods_column_view();
        let detail_related_pods_stack = gtk::Stack::builder()
            .hhomogeneous(false)
            .vhomogeneous(false)
            .build();
        let detail_related_pods_message = adw::StatusPage::builder()
            .title("Pods are shown for Deployments")
            .description("Open a Deployment to inspect its related Pods.")
            .icon_name("dialog-information-symbolic")
            .build();
        detail_related_pods_message.add_css_class("compact");
        let detail_log_container_dropdown = gtk::DropDown::from_strings(&["No containers"]);
        detail_log_container_dropdown.set_sensitive(false);
        let detail_log_follow_check = gtk::CheckButton::builder().label("Follow").build();
        detail_log_follow_check.set_active(true);
        detail_log_follow_check.set_sensitive(false);
        let detail_log_timestamps_check = gtk::CheckButton::builder().label("Timestamps").build();
        detail_log_timestamps_check.set_active(false);
        detail_log_timestamps_check.set_sensitive(false);
        let detail_log_start_button = gtk::Button::builder()
            .label("Start")
            .icon_name("media-playback-start-symbolic")
            .tooltip_text("Start log streaming")
            .sensitive(false)
            .build();
        let detail_log_stop_button = gtk::Button::builder()
            .label("Stop")
            .icon_name("media-playback-stop-symbolic")
            .tooltip_text("Stop log streaming")
            .sensitive(false)
            .build();
        let detail_log_clear_button = gtk::Button::builder()
            .label("Clear")
            .icon_name("edit-clear-symbolic")
            .tooltip_text("Clear visible logs")
            .build();
        let detail_log_download_button = gtk::Button::builder()
            .icon_name("document-save-as-symbolic")
            .tooltip_text("Save logs to a local file")
            .build();
        let detail_log_status_label = gtk::Label::builder()
            .label("Logs are available for Pods.")
            .xalign(0.0)
            .hexpand(true)
            .build();
        let detail_log_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        setup_log_highlighting(&detail_log_buffer);
        let detail_log_view = gtk::TextView::with_buffer(&detail_log_buffer);
        let detail_port_local_spin = gtk::SpinButton::with_range(0.0, 65535.0, 1.0);
        detail_port_local_spin.set_numeric(true);
        detail_port_local_spin.set_value(0.0);
        detail_port_local_spin.set_tooltip_text(Some("Local port, or 0 for an automatic port"));
        let detail_port_remote_spin = gtk::SpinButton::with_range(1.0, 65535.0, 1.0);
        detail_port_remote_spin.set_numeric(true);
        detail_port_remote_spin.set_value(8080.0);
        detail_port_remote_spin.set_tooltip_text(Some("Pod port to forward"));
        let detail_port_start_button = gtk::Button::builder()
            .label("Start")
            .icon_name("media-playback-start-symbolic")
            .tooltip_text("Start port forwarding")
            .sensitive(false)
            .build();
        let detail_port_stop_button = gtk::Button::builder()
            .label("Stop")
            .icon_name("media-playback-stop-symbolic")
            .tooltip_text("Stop port forwarding")
            .sensitive(false)
            .build();
        let detail_port_status_label = gtk::Label::builder()
            .label("Port forwarding is available for Pods.")
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
            related_pods_view: &detail_related_pods_view,
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

        let setup_name_entry = adw::EntryRow::builder().title("Name").hexpand(true).build();
        let setup_server_entry = adw::EntryRow::builder()
            .title("API Server")
            .hexpand(true)
            .build();
        let setup_token_entry = adw::PasswordEntryRow::builder()
            .title("Bearer Token")
            .hexpand(true)
            .build();
        let setup_ca_entry = adw::EntryRow::builder()
            .title("CA Data")
            .hexpand(true)
            .build();
        let setup_insecure_check = adw::SwitchRow::builder()
            .title("Skip TLS Verification")
            .build();
        let setup_button = gtk::Button::builder()
            .label("Add Cluster")
            .sensitive(true)
            .build();
        setup_button.add_css_class("suggested-action");
        let cluster_token_title_label = gtk::Label::new(Some("Connect with token"));
        let cluster_token_back_button = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .build();
        let custom_namespace_dialog =
            build_custom_namespace_dialog(&custom_namespace_entry, &custom_namespace_button);
        let rename_namespace_dialog =
            build_rename_namespace_dialog(&rename_namespace_entry, &rename_namespace_button);
        let project_dialog = build_project_dialog(
            &project_name_entry,
            &project_create_button,
            &project_dialog_description,
        );
        let create_yaml_buffer = sourceview5::Buffer::new(None);
        let create_yaml_error_label = gtk::Label::new(None);
        setup_yaml_buffer(&create_yaml_buffer, &create_yaml_error_label);
        let create_yaml_apply_button = gtk::Button::builder().label("Create").build();
        create_yaml_apply_button.add_css_class("suggested-action");
        let create_yaml_dialog = build_create_yaml_dialog(
            &create_yaml_buffer,
            &create_yaml_apply_button,
            &create_yaml_error_label,
        );

        let cluster_dialog_stack = gtk::Stack::new();
        let cluster_dialog = build_cluster_dialog(
            ClusterDialogWidgets {
                stack: &cluster_dialog_stack,
                name_entry: &setup_name_entry,
                server_entry: &setup_server_entry,
                token_entry: &setup_token_entry,
                ca_entry: &setup_ca_entry,
                insecure_check: &setup_insecure_check,
                add_button: &setup_button,
                title_label: &cluster_token_title_label,
                back_button: &cluster_token_back_button,
            },
            sender.clone(),
        );

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
            .tooltip_text("Show Sidebar")
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
        });
        let content = build_content(ContentWidgets {
            sidebar_toggle_button: &sidebar_toggle_button,
            detail_back_button: &detail_back_button,
            delete_button: &detail_delete_button,
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

        project_list.connect_row_activated({
            let sender = sender.clone();
            move |_, row| sender.input(AppMsg::ProjectChanged(row.index() as u32))
        });
        cluster_list.connect_row_activated({
            let sender = sender.clone();
            move |_, row| sender.input(AppMsg::ClusterChanged(row.index() as u32))
        });
        cluster_back_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowClusters)
        });
        // Backing actions for the hamburger menus. Registered on the window
        // ("win." prefix); the accels are what the popover shows on the
        // right of each item.
        type MenuAction = (&'static str, &'static [&'static str], fn() -> AppMsg);
        let menu_actions: [MenuAction; 5] = [
            ("cluster-edit", &["<primary>E"], || {
                AppMsg::EditCurrentCluster
            }),
            ("cluster-remove", &["<primary><shift>Delete"], || {
                AppMsg::RemoveClusterFromProject
            }),
            ("project-rename", &["F2"], || {
                AppMsg::ShowRenameProjectDialog
            }),
            ("project-duplicate", &["<primary>D"], || {
                AppMsg::DuplicateProject
            }),
            ("project-delete", &["<primary>Delete"], || {
                AppMsg::DeleteProject
            }),
        ];
        let application = relm4::main_application();
        for (name, accels, message) in menu_actions {
            let action = gtk::gio::SimpleAction::new(name, None);
            action.connect_activate({
                let sender = sender.clone();
                move |_, _| sender.input(message())
            });
            root.add_action(&action);
            application.set_accels_for_action(&format!("win.{name}"), accels);
        }
        add_cluster_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowAddClusterDialog)
        });
        clusters_empty_add_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowAddClusterDialog)
        });
        import_cluster_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowImportFile)
        });
        cluster_refresh_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::RefreshClusters)
        });
        namespace_list.connect_row_activated({
            let sender = sender.clone();
            move |_, row| sender.input(AppMsg::NamespaceChanged(row.index() as u32))
        });
        add_project_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowAddProjectDialog)
        });
        projects_empty_add_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowAddProjectDialog)
        });
        custom_namespace_entry.connect_entry_activated({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::CustomNamespaceEntered)
        });
        custom_namespace_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::CustomNamespaceEntered)
        });
        rename_namespace_entry.connect_entry_activated({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::RenameNamespaceConfirmed)
        });
        rename_namespace_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::RenameNamespaceConfirmed)
        });
        status_filter_list.connect_child_activated({
            let sender = sender.clone();
            move |_, child| sender.input(AppMsg::StatusFilterChanged(child.index() as u32))
        });
        column_filter_list.connect_child_activated({
            let sender = sender.clone();
            move |_, child| sender.input(AppMsg::ObjectColumnToggled(child.index() as u32))
        });
        project_name_entry.connect_entry_activated({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::AddProject)
        });
        project_create_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::AddProject)
        });
        search_entry.connect_search_changed({
            let sender = sender.clone();
            move |entry| sender.input(AppMsg::SearchChanged(entry.text().to_string()))
        });
        object_view.connect_activate({
            let sender = sender.clone();
            move |_, position| sender.input(AppMsg::ObjectActivated(position as i32))
        });
        for (table_column, view_column) in &object_columns {
            super::widgets::connect_object_column_persistence(
                view_column,
                *table_column,
                sender.clone(),
            );
        }
        create_yaml_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowCreateYamlDialog)
        });
        create_yaml_apply_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::CreateYaml)
        });
        detail_related_pods_view.connect_activate({
            let sender = sender.clone();
            move |_, position| sender.input(AppMsg::RelatedPodActivated(position as i32))
        });
        detail_back_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::BackToObjects)
        });
        projects_home_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowProjects)
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
        refresh_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::Refresh)
        });
        setup_button.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::AddCluster)
        });

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
            status: String::from("Loading kubeconfig..."),
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
            object_store,
            object_sorted,
            object_columns,
            object_list_stack,
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
                terminal_button: detail_terminal_button,
                yaml_buffer: detail_yaml_buffer,
                events_list: detail_events_list,
                conditions_list: detail_conditions_list,
                related_pods_store: detail_related_pods_store,
                related_pods_sorted: detail_related_pods_sorted,
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
