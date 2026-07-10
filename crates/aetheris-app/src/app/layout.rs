use super::widgets::*;
use super::*;

pub(super) struct SidebarWidgets<'a> {
    pub(super) cluster_title: &'a gtk::Label,
    pub(super) cluster_back_button: &'a gtk::Button,
    pub(super) cluster_menu_button: &'a gtk::MenuButton,
    pub(super) namespace_menu_button: &'a gtk::MenuButton,
    pub(super) resource_list: &'a gtk::ListBox,
    pub(super) favorite_object_list: &'a gtk::ListBox,
}

pub(super) fn build_sidebar(widgets: SidebarWidgets<'_>) -> adw::NavigationPage {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.pack_start(widgets.cluster_back_button);
    header.set_title_widget(Some(widgets.cluster_title));
    header.pack_end(widgets.cluster_menu_button);
    toolbar.add_top_bar(&header);

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 12);
    outer.set_margin_all(12);

    let container = gtk::Box::new(gtk::Orientation::Vertical, 12);

    let namespace_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    namespace_group.append(&section_title(&tr("Namespace")));
    widgets.namespace_menu_button.set_hexpand(true);
    namespace_group.append(widgets.namespace_menu_button);

    container.append(&namespace_group);

    let resources_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    resources_page.append(widgets.resource_list);

    let favorites_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    favorites_page.append(widgets.favorite_object_list);

    let sidebar_stack = gtk::Stack::builder()
        .hhomogeneous(false)
        .vhomogeneous(false)
        .build();
    sidebar_stack.add_named(&resources_page, Some("resources"));
    sidebar_stack.add_named(&favorites_page, Some("favorites"));

    let toggle_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let resources_toggle = gtk::ToggleButton::builder()
        .icon_name("view-list-symbolic")
        .tooltip_text(tr("Resources"))
        .hexpand(true)
        .active(true)
        .css_classes(["flat"])
        .build();
    let favorites_toggle = gtk::ToggleButton::builder()
        .icon_name("aetheris-object-favorite-symbolic")
        .tooltip_text(tr("Favorites"))
        .hexpand(true)
        .css_classes(["flat"])
        .build();
    toggle_box.append(&resources_toggle);
    toggle_box.append(&favorites_toggle);

    resources_toggle.connect_clicked({
        let resources_toggle = resources_toggle.clone();
        move |_| resources_toggle.set_active(true)
    });
    resources_toggle.connect_toggled({
        let favorites_toggle = favorites_toggle.clone();
        let sidebar_stack = sidebar_stack.clone();
        move |button| {
            if button.is_active() {
                favorites_toggle.set_active(false);
                sidebar_stack.set_visible_child_name("resources");
            }
        }
    });
    favorites_toggle.connect_clicked({
        let favorites_toggle = favorites_toggle.clone();
        move |_| favorites_toggle.set_active(true)
    });
    favorites_toggle.connect_toggled({
        let resources_toggle = resources_toggle.clone();
        let sidebar_stack = sidebar_stack.clone();
        move |button| {
            if button.is_active() {
                resources_toggle.set_active(false);
                sidebar_stack.set_visible_child_name("favorites");
            }
        }
    });

    let objects_group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    objects_group.append(&toggle_box);
    objects_group.append(&sidebar_stack);
    container.append(&objects_group);

    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .min_content_width(180)
        .propagate_natural_height(false)
        .build();
    scrolled.set_child(Some(&container));
    outer.append(&scrolled);
    toolbar.set_content(Some(&outer));

    adw::NavigationPage::new(&toolbar, "Aetheris")
}

pub(super) struct ContentWidgets<'a> {
    pub(super) sidebar_toggle_button: &'a gtk::ToggleButton,
    pub(super) detail_back_button: &'a gtk::Button,
    pub(super) delete_button: &'a gtk::Button,
    pub(super) favorite_button: &'a gtk::Button,
    pub(super) create_yaml_button: &'a gtk::Button,
    pub(super) refresh_button: &'a gtk::Button,
    pub(super) terminal_button: &'a gtk::Button,
    pub(super) search_entry: &'a gtk::SearchEntry,
    pub(super) status_filter_list: &'a gtk::FlowBox,
    pub(super) column_filter_list: &'a gtk::FlowBox,
    pub(super) title: &'a gtk::Label,
    pub(super) header_stack: &'a gtk::Stack,
    pub(super) content_stack: &'a gtk::Stack,
    pub(super) status_label: &'a gtk::Label,
    pub(super) spinner: &'a gtk::Spinner,
    pub(super) object_view: &'a gtk::ColumnView,
    pub(super) object_list_stack: &'a gtk::Stack,
    pub(super) detail_page: &'a gtk::Box,
}

pub(super) fn build_content(widgets: ContentWidgets<'_>) -> adw::NavigationPage {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.pack_start(widgets.sidebar_toggle_button);
    header.pack_start(widgets.detail_back_button);
    header.pack_start(widgets.refresh_button);
    header.pack_start(widgets.create_yaml_button);

    let search_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    search_box.add_css_class("linked");
    search_box.add_css_class("search-toolbar");
    widgets.search_entry.set_hexpand(true);
    search_box.append(widgets.search_entry);

    let filter_button = gtk::MenuButton::builder()
        .icon_name(available_icon_name(
            "nautilus-search-filters-symbolic",
            "preferences-system-symbolic",
        ))
        .tooltip_text(tr("Filters"))
        .build();
    filter_button.set_popover(Some(&filter_popover(
        widgets.status_filter_list,
        widgets.column_filter_list,
    )));
    search_box.append(&filter_button);

    widgets.header_stack.add_named(&search_box, Some("search"));
    widgets.header_stack.add_named(widgets.title, Some("title"));
    widgets.header_stack.set_visible_child_name("search");
    header.set_title_widget(Some(widgets.header_stack));
    let detail_actions = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    detail_actions.set_valign(gtk::Align::Center);
    detail_actions.set_halign(gtk::Align::End);
    detail_actions.append(widgets.terminal_button);
    detail_actions.append(widgets.favorite_button);
    detail_actions.append(widgets.delete_button);
    header.pack_end(&detail_actions);
    toolbar.add_top_bar(&header);

    let list_page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    list_page.set_margin_top(12);
    list_page.set_margin_bottom(12);
    list_page.set_margin_start(12);
    list_page.set_margin_end(12);

    let status = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    status.append(widgets.status_label);
    status.append(widgets.spinner);
    list_page.append(&status);

    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        // Columns are user-resizable now, so the table can legitimately
        // grow wider than the window (e.g. after widening Name and a
        // couple of data columns) — a horizontal scrollbar is how you
        // reach whatever's pushed past the right edge instead of it just
        // being clipped and unreachable.
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .css_classes(["aetheris-table-frame"])
        .build();
    // The ColumnView must be the ScrolledWindow's direct child: it
    // implements GtkScrollable, and only then does it realize widgets for
    // just the on-screen rows. Wrapped in a plain Box it would be allocated
    // its full natural height and materialize every row at once.
    scrolled.set_child(Some(widgets.object_view));

    let empty_page = adw::StatusPage::builder()
        .title(tr("No objects"))
        .description(tr(
            "The selected resource has no objects or could not be loaded.",
        ))
        .icon_name("edit-find-symbolic")
        .build();
    empty_page.add_css_class("compact");

    widgets
        .object_list_stack
        .add_named(&scrolled, Some("table"));
    widgets
        .object_list_stack
        .add_named(&empty_page, Some("empty"));
    widgets.object_list_stack.set_visible_child_name("empty");
    widgets.object_list_stack.set_vexpand(true);
    list_page.append(widgets.object_list_stack);

    widgets.content_stack.add_named(&list_page, Some("list"));
    widgets
        .content_stack
        .add_named(widgets.detail_page, Some("detail"));
    widgets.content_stack.set_visible_child_name("list");
    toolbar.set_content(Some(widgets.content_stack));

    adw::NavigationPage::new(&toolbar, "Objects")
}

pub(super) fn filter_popover(
    status_filter_list: &gtk::FlowBox,
    column_filter_list: &gtk::FlowBox,
) -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_size_request(380, -1);

    let title = gtk::Label::builder()
        .label(tr("Status"))
        .xalign(0.0)
        .css_classes(["caption-heading"])
        .build();
    content.append(&title);

    content.append(status_filter_list);

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    content.append(&separator);

    let columns_title = gtk::Label::builder()
        .label(tr("Columns"))
        .xalign(0.0)
        .css_classes(["caption-heading"])
        .build();
    content.append(&columns_title);
    content.append(column_filter_list);

    popover.set_child(Some(&content));
    popover
}

pub(super) fn build_projects_page(
    project_list: &gtk::ListBox,
    add_button: &gtk::Button,
    content_stack: &gtk::Stack,
    empty_page: &adw::StatusPage,
) -> adw::ToolbarView {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.pack_end(add_button);
    toolbar.add_top_bar(&header);

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(24);
    outer.set_margin_bottom(24);
    outer.set_margin_start(12);
    outer.set_margin_end(12);

    let clamp = adw::Clamp::builder()
        .maximum_size(720)
        .tightening_threshold(560)
        .build();
    clamp.add_css_class("content-clamp");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.set_valign(gtk::Align::Start);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    let heading = gtk::Label::new(Some(&tr("Select a Project")));
    heading.set_xalign(0.0);
    heading.add_css_class("title-1");
    content.append(&heading);

    let subtitle = gtk::Label::new(Some(&tr(
        "Projects keep clusters, namespaces and resources separated by environment, team or company.",
    )));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("dim-label");
    content.append(&subtitle);

    let group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    group.append(&section_title(&tr("Projects")));
    group.append(project_list);
    content.append(&group);

    clamp.set_child(Some(&content));

    content_stack.add_named(&clamp, Some("content"));
    content_stack.add_named(empty_page, Some("empty"));
    content_stack.set_visible_child_name("content");

    let center = gtk::CenterBox::new();
    center.set_hexpand(true);
    center.set_vexpand(true);
    center.set_center_widget(Some(content_stack));

    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    scrolled.set_child(Some(&center));
    outer.append(&scrolled);

    toolbar.set_content(Some(&outer));
    toolbar
}

pub(super) struct ClustersPageWidgets<'a> {
    pub(super) projects_home_button: &'a gtk::Button,
    pub(super) project_title: &'a gtk::Label,
    pub(super) project_menu_button: &'a gtk::MenuButton,
    pub(super) refresh_button: &'a gtk::Button,
    pub(super) cluster_list: &'a gtk::ListBox,
    pub(super) add_cluster_button: &'a gtk::Button,
    pub(super) import_cluster_button: &'a gtk::Button,
    pub(super) content_stack: &'a gtk::Stack,
    pub(super) empty_page: &'a adw::StatusPage,
}

pub(super) fn build_clusters_page(widgets: ClustersPageWidgets<'_>) -> adw::ToolbarView {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.pack_start(widgets.projects_home_button);
    header.pack_start(widgets.refresh_button);
    header.set_title_widget(Some(widgets.project_title));
    widgets.add_cluster_button.add_css_class("flat");
    widgets.import_cluster_button.add_css_class("flat");
    header.pack_end(widgets.project_menu_button);
    header.pack_end(widgets.add_cluster_button);
    toolbar.add_top_bar(&header);
    let cluster_list = widgets.cluster_list;

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(24);
    outer.set_margin_bottom(24);
    outer.set_margin_start(12);
    outer.set_margin_end(12);

    let clamp = adw::Clamp::builder()
        .maximum_size(720)
        .tightening_threshold(560)
        .build();
    clamp.add_css_class("content-clamp");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.set_valign(gtk::Align::Start);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    let heading = gtk::Label::new(Some(&tr("Clusters")));
    heading.set_xalign(0.0);
    heading.add_css_class("title-1");
    content.append(&heading);

    let subtitle = gtk::Label::new(Some(&tr(
        "Pick a cluster to browse, or add a new one to this project.",
    )));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("dim-label");
    content.append(&subtitle);

    let group = gtk::Box::new(gtk::Orientation::Vertical, 8);
    group.append(&section_title(&tr("Clusters")));
    group.append(cluster_list);
    content.append(&group);

    clamp.set_child(Some(&content));

    widgets.content_stack.add_named(&clamp, Some("content"));
    widgets
        .content_stack
        .add_named(widgets.empty_page, Some("empty"));
    widgets.content_stack.set_visible_child_name("content");

    let center = gtk::CenterBox::new();
    center.set_hexpand(true);
    center.set_vexpand(true);
    center.set_center_widget(Some(widgets.content_stack));

    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    scrolled.set_child(Some(&center));
    outer.append(&scrolled);

    toolbar.set_content(Some(&outer));
    toolbar
}
