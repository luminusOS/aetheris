use super::super::dialogs::*;
use super::super::widgets::{
    append_filler_column, connect_object_column_persistence, connect_sorted_header_highlight,
    object_column_sorter, object_data_column_factory, object_name_column_factory,
    rebuild_column_filter_list, rebuild_status_filter_list,
};
use super::super::yaml::*;
use super::super::*;

pub(super) struct ObjectTableWidgets {
    pub(super) search_entry: gtk::SearchEntry,
    pub(super) status_filter_list: gtk::FlowBox,
    pub(super) column_filter_list: gtk::FlowBox,
    pub(super) create_yaml_button: gtk::Button,
    pub(super) refresh_button: gtk::Button,
    pub(super) create_yaml_dialog: adw::Dialog,
    pub(super) create_yaml_buffer: sourceview5::Buffer,
    pub(super) create_yaml_apply_button: gtk::Button,
    pub(super) object_store: gtk::gio::ListStore,
    pub(super) object_view: gtk::ColumnView,
    pub(super) object_sorted: gtk::SortListModel,
    pub(super) object_columns: Vec<(ObjectTableColumn, gtk::ColumnViewColumn)>,
    pub(super) object_list_stack: gtk::Stack,
}

/// Builds the search bar, status/column filter chips, the object
/// `ColumnView` (with its per-column factories/sorters), and the
/// create-from-YAML dialog, and wires their signals to `AppMsg`.
pub(super) fn build(sender: &ComponentSender<App>) -> ObjectTableWidgets {
    // width-chars is the entry's *minimum* width and the header bar
    // passes it straight up to the window: at 28 chars the whole content
    // pane bottomed out around 589px and the window refused to shrink
    // further (Adwaita then warns the toast overlay exceeds the window).
    // Keep the minimum tiny and let hexpand grow it into whatever the
    // header actually has.
    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search"))
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
    let create_yaml_button = gtk::Button::builder()
        .label(tr("Create"))
        .icon_name("document-new-symbolic")
        .tooltip_text(tr("Create object from YAML"))
        .sensitive(false)
        .build();
    let refresh_button = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(tr("Refresh resources"))
        .sensitive(false)
        .build();

    let object_store = gtk::gio::ListStore::new::<gtk::glib::BoxedAnyObject>();
    let object_view = gtk::ColumnView::builder()
        .single_click_activate(true)
        .reorderable(false)
        .build();
    object_view.add_css_class("aetheris-table");
    object_view.set_vexpand(true);

    let mut object_columns: Vec<(ObjectTableColumn, gtk::ColumnViewColumn)> = Vec::new();
    let name_column =
        gtk::ColumnViewColumn::new(Some(&tr("Name")), Some(object_name_column_factory()));
    name_column.set_resizable(true);
    name_column.set_fixed_width(OBJECT_NAME_WIDTH);
    object_view.append_column(&name_column);
    object_columns.push((ObjectTableColumn::Name, name_column));
    for column in ObjectColumn::ALL {
        let view_column = gtk::ColumnViewColumn::new(
            Some(&column.label()),
            Some(object_data_column_factory(column)),
        );
        view_column.set_resizable(true);
        view_column.set_fixed_width(column.default_width());
        view_column.set_sorter(object_column_sorter(column).as_ref());
        object_view.append_column(&view_column);
        object_columns.push((ObjectTableColumn::Data(column), view_column));
    }
    append_filler_column(&object_view);
    // Header-click sorting reorders the view, not the store, so
    // activation positions must be resolved against this sorted model.
    let object_sorted = gtk::SortListModel::new(Some(object_store.clone()), object_view.sorter());
    object_view.set_model(Some(&gtk::NoSelection::new(Some(object_sorted.clone()))));
    connect_sorted_header_highlight(&object_view);

    let object_list_stack = gtk::Stack::builder()
        .hhomogeneous(false)
        .vhomogeneous(false)
        .build();

    let create_yaml_buffer = sourceview5::Buffer::new(None);
    let create_yaml_error_label = gtk::Label::new(None);
    setup_yaml_buffer(&create_yaml_buffer, &create_yaml_error_label);
    let create_yaml_apply_button = gtk::Button::builder().label(tr("Create")).build();
    create_yaml_apply_button.add_css_class("suggested-action");
    let create_yaml_dialog = build_create_yaml_dialog(
        &create_yaml_buffer,
        &create_yaml_apply_button,
        &create_yaml_error_label,
    );

    status_filter_list.connect_child_activated({
        let sender = sender.clone();
        move |_, child| sender.input(AppMsg::StatusFilterChanged(child.index() as u32))
    });
    column_filter_list.connect_child_activated({
        let sender = sender.clone();
        move |_, child| sender.input(AppMsg::ObjectColumnToggled(child.index() as u32))
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
        connect_object_column_persistence(view_column, *table_column, sender.clone());
    }
    create_yaml_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowCreateYamlDialog)
    });
    create_yaml_apply_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::CreateYaml)
    });
    refresh_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::Refresh)
    });

    ObjectTableWidgets {
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
    }
}
