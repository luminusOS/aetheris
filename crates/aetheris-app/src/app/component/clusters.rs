use super::super::dialogs::*;
use super::super::*;

pub(super) struct ClustersWidgets {
    pub(super) context_selector_label: gtk::Label,
    pub(super) cluster_back_button: gtk::Button,
    pub(super) cluster_menu_button: gtk::MenuButton,
    pub(super) cluster_refresh_button: gtk::Button,
    pub(super) cluster_list: gtk::ListBox,
    pub(super) add_cluster_button: gtk::Button,
    pub(super) import_cluster_button: gtk::Button,
    pub(super) clusters_empty_page: adw::StatusPage,
    pub(super) clusters_content_stack: gtk::Stack,
    pub(super) resource_list: gtk::ListBox,
    pub(super) favorite_object_list: gtk::ListBox,
    pub(super) cluster_dialog_stack: gtk::Stack,
    pub(super) setup_name_entry: adw::EntryRow,
    pub(super) setup_server_entry: adw::EntryRow,
    pub(super) setup_token_entry: adw::PasswordEntryRow,
    pub(super) setup_ca_entry: adw::EntryRow,
    pub(super) setup_insecure_check: adw::SwitchRow,
    pub(super) setup_button: gtk::Button,
    pub(super) cluster_token_title_label: gtk::Label,
    pub(super) cluster_token_back_button: gtk::Button,
    pub(super) cluster_dialog: adw::Dialog,
}

/// Builds the clusters page (list, empty state, options menu), the sidebar's
/// resource/favorite lists, and the add/edit-cluster setup dialog, and wires
/// their signals to `AppMsg`.
pub(super) fn build(sender: &ComponentSender<App>) -> ClustersWidgets {
    let context_selector_label = gtk::Label::new(Some(&tr("No cluster")));
    context_selector_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    context_selector_label.set_max_width_chars(22);
    let cluster_back_button = gtk::Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text(tr("Back to clusters"))
        .build();
    cluster_back_button.add_css_class("flat");
    let cluster_menu = gtk::gio::Menu::new();
    cluster_menu.append(Some(&tr("Edit Cluster...")), Some("win.cluster-edit"));
    cluster_menu.append(Some(&tr("Remove from Project")), Some("win.cluster-remove"));
    let cluster_menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(tr("Cluster options"))
        .menu_model(&cluster_menu)
        .build();
    let cluster_refresh_button = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(tr("Refresh cluster health"))
        .build();
    cluster_refresh_button.add_css_class("flat");
    let cluster_list = gtk::ListBox::new();
    cluster_list.set_hexpand(true);
    cluster_list.add_css_class("boxed-list");
    cluster_list.set_selection_mode(gtk::SelectionMode::None);
    let add_cluster_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(tr("Add cluster"))
        .build();
    let import_cluster_button = gtk::Button::builder().label(tr("Import")).build();
    let clusters_empty_add_button = gtk::Button::builder()
        .child(
            &adw::ButtonContent::builder()
                .icon_name("list-add-symbolic")
                .label(tr("Add Cluster"))
                .build(),
        )
        .halign(gtk::Align::Center)
        .build();
    clusters_empty_add_button.add_css_class("suggested-action");
    let clusters_empty_page = adw::StatusPage::builder()
        .icon_name("network-server-symbolic")
        .title(tr("No Clusters Yet"))
        .description(tr("Add a cluster to start browsing this project."))
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();
    clusters_empty_page.set_child(Some(&clusters_empty_add_button));
    let clusters_content_stack = gtk::Stack::new();

    let resource_list = gtk::ListBox::new();
    resource_list.add_css_class("boxed-list");
    resource_list.set_selection_mode(gtk::SelectionMode::None);
    let favorite_object_list = gtk::ListBox::new();
    favorite_object_list.add_css_class("boxed-list");
    favorite_object_list.set_selection_mode(gtk::SelectionMode::None);

    let setup_name_entry = adw::EntryRow::builder()
        .title(tr("Name"))
        .hexpand(true)
        .build();
    let setup_server_entry = adw::EntryRow::builder()
        .title(tr("API Server"))
        .hexpand(true)
        .build();
    let setup_token_entry = adw::PasswordEntryRow::builder()
        .title(tr("Bearer Token"))
        .hexpand(true)
        .build();
    let setup_ca_entry = adw::EntryRow::builder()
        .title(tr("CA Certificate"))
        .hexpand(true)
        .build();
    let setup_ca_file_button = gtk::Button::builder()
        .icon_name("document-open-symbolic")
        .build();
    let setup_insecure_check = adw::SwitchRow::builder()
        .title(tr("Skip TLS Verification"))
        .build();
    let setup_button = gtk::Button::builder()
        .label(tr("Add Cluster"))
        .sensitive(true)
        .build();
    setup_button.add_css_class("suggested-action");
    let cluster_token_title_label = gtk::Label::new(Some(&tr("Connect with token")));
    let cluster_token_back_button = gtk::Button::builder()
        .icon_name("go-previous-symbolic")
        .build();
    let cluster_dialog_stack = gtk::Stack::new();
    let cluster_dialog = build_cluster_dialog(
        ClusterDialogWidgets {
            stack: &cluster_dialog_stack,
            name_entry: &setup_name_entry,
            server_entry: &setup_server_entry,
            token_entry: &setup_token_entry,
            ca_entry: &setup_ca_entry,
            ca_file_button: &setup_ca_file_button,
            insecure_check: &setup_insecure_check,
            add_button: &setup_button,
            title_label: &cluster_token_title_label,
            back_button: &cluster_token_back_button,
        },
        sender.clone(),
    );

    cluster_list.connect_row_activated({
        let sender = sender.clone();
        move |_, row| sender.input(AppMsg::ClusterChanged(row.index() as u32))
    });
    cluster_back_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowClusters)
    });
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
    setup_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::AddCluster)
    });
    setup_ca_file_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowCaFile)
    });

    ClustersWidgets {
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
    }
}
