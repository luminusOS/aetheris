use super::commands::*;
use super::layout::*;
use super::*;

mod clusters;
mod detail_pane;
mod namespaces;
mod object_table;
mod projects;
mod window_actions;
use clusters::ClustersWidgets;
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
        let (detail, detail_page) = detail_pane::build(&sender);

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
            detail_back_button: &detail.back_button,
            delete_button: &detail.delete_button,
            favorite_button: &detail.favorite_button,
            create_yaml_button: &create_yaml_button,
            refresh_button: &refresh_button,
            terminal_button: &detail.terminal_button,
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
            detail,
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
