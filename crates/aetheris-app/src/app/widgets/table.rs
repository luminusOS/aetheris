use super::filters::status_chip;
use super::*;

const RELATED_POD_COLUMNS: [ObjectColumn; 4] = [
    ObjectColumn::Image,
    ObjectColumn::Namespace,
    ObjectColumn::Api,
    ObjectColumn::Age,
];

/// The virtualized related-Pods table for the detail page's "Pods" tab:
/// same cell factories as the main object table, default widths, no
/// persistence. Returns the sorted model too — activation positions are
/// indices into it, not into the unsorted store.
pub(crate) fn related_pods_column_view()
-> (gtk::ColumnView, gtk::gio::ListStore, gtk::SortListModel) {
    let store = gtk::gio::ListStore::new::<gtk::glib::BoxedAnyObject>();
    let view = gtk::ColumnView::builder()
        .single_click_activate(true)
        .reorderable(false)
        .build();
    view.add_css_class("aetheris-table");
    view.set_vexpand(true);

    let name_column =
        gtk::ColumnViewColumn::new(Some(&tr("Name")), Some(object_name_column_factory()));
    name_column.set_resizable(true);
    name_column.set_fixed_width(OBJECT_NAME_WIDTH);
    view.append_column(&name_column);
    for column in RELATED_POD_COLUMNS {
        let view_column = gtk::ColumnViewColumn::new(
            Some(&column.label()),
            Some(object_data_column_factory(column)),
        );
        view_column.set_resizable(true);
        view_column.set_fixed_width(column.default_width());
        view_column.set_sorter(object_column_sorter(column).as_ref());
        view.append_column(&view_column);
    }
    append_filler_column(&view);

    let sorted = gtk::SortListModel::new(Some(store.clone()), view.sorter());
    view.set_model(Some(&gtk::NoSelection::new(Some(sorted.clone()))));
    connect_sorted_header_highlight(&view);
    (view, store, sorted)
}

/// Trailing zero-content column that soaks up leftover width, so the
/// header background always reaches the table's right edge instead of
/// stopping after the last real column.
pub(crate) fn append_filler_column(view: &gtk::ColumnView) {
    let filler = gtk::ColumnViewColumn::new(None, None::<gtk::ListItemFactory>);
    filler.set_expand(true);
    view.append_column(&filler);
}

/// Mirrors the active sort column onto its header button via a "sorted"
/// CSS class. GTK itself only draws the small direction arrow and exposes
/// no styleable state for the sorted column, so this walks the header's
/// buttons (one per column, same order) whenever the view's sorter fires.
pub(crate) fn connect_sorted_header_highlight(view: &gtk::ColumnView) {
    let Some(sorter) = view.sorter() else {
        return;
    };
    let view = view.downgrade();
    sorter.connect_changed(move |sorter, _| {
        let Some(view) = view.upgrade() else {
            return;
        };
        let Some(sorter) = sorter.downcast_ref::<gtk::ColumnViewSorter>() else {
            return;
        };
        let primary = sorter.primary_sort_column();
        let Some(header) = column_view_header(&view) else {
            return;
        };
        let columns = view.columns();
        let mut index = 0;
        let mut child = header.first_child();
        while let Some(button) = child {
            child = button.next_sibling();
            let column = columns.item(index).and_downcast::<gtk::ColumnViewColumn>();
            if column.is_some() && column == primary {
                button.add_css_class("sorted");
            } else {
                button.remove_css_class("sorted");
            }
            index += 1;
        }
    });
}

fn column_view_header(view: &gtk::ColumnView) -> Option<gtk::Widget> {
    let mut child = view.first_child();
    while let Some(widget) = child {
        if widget.css_name() == "header" {
            return Some(widget);
        }
        child = widget.next_sibling();
    }
    None
}

pub(crate) fn object_column_sorter(column: ObjectColumn) -> Option<gtk::CustomSorter> {
    match column {
        ObjectColumn::Image => Some(summary_sorter(|a, b| {
            super::super::utils::pod_main_image(&a.images)
                .cmp(&super::super::utils::pod_main_image(&b.images))
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Namespace => Some(summary_sorter(|a, b| {
            a.namespace
                .cmp(&b.namespace)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Target => Some(summary_sorter(|a, b| {
            a.service_target
                .cmp(&b.service_target)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Selector => Some(summary_sorter(|a, b| {
            a.service_selector
                .cmp(&b.service_selector)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::IngressClass => Some(summary_sorter(|a, b| {
            a.ingress_class
                .cmp(&b.ingress_class)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Cpu | ObjectColumn::Memory => Some(summary_sorter(move |a, b| {
            // Usage percentage is the primary key; raw quantity only breaks
            // ties among objects with no percentage (no requests set), and
            // `None` (no metrics sample) groups at one end.
            let (a_ratio, a_raw) = metric_sort_key(a, column);
            let (b_ratio, b_raw) = metric_sort_key(b, column);
            a_ratio
                .cmp(&b_ratio)
                .then_with(|| {
                    a_raw
                        .partial_cmp(&b_raw)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.name.cmp(&b.name))
        })),
        _ => None,
    }
}

fn metric_sort_key(object: &ObjectSummary, column: ObjectColumn) -> (Option<u32>, Option<f64>) {
    let Some(usage) = object.metrics.as_ref() else {
        return (None, None);
    };
    let (ratio, raw) = match column {
        ObjectColumn::Cpu => (usage.cpu_ratio.as_ref(), &usage.cpu),
        ObjectColumn::Memory => (usage.memory_ratio.as_ref(), &usage.memory),
        _ => return (None, None),
    };
    (
        ratio.map(|ratio| ratio.basis_points),
        super::super::utils::parse_quantity(raw),
    )
}

fn summary_sorter(
    compare: impl Fn(&ObjectSummary, &ObjectSummary) -> std::cmp::Ordering + 'static,
) -> gtk::CustomSorter {
    gtk::CustomSorter::new(move |a, b| {
        let (Some(a), Some(b)) = (
            a.downcast_ref::<gtk::glib::BoxedAnyObject>(),
            b.downcast_ref::<gtk::glib::BoxedAnyObject>(),
        ) else {
            return gtk::Ordering::Equal;
        };
        compare(&a.borrow::<ObjectSummary>(), &b.borrow::<ObjectSummary>()).into()
    })
}

pub(crate) fn connect_object_column_persistence(
    view_column: &gtk::ColumnViewColumn,
    table_column: ObjectTableColumn,
    sender: ComponentSender<App>,
) {
    view_column.connect_fixed_width_notify(move |view_column| {
        let width = view_column.fixed_width();
        let clamped = clamp_table_column_width(table_column, width);
        if clamped != width {
            view_column.set_fixed_width(clamped);
            return;
        }
        sender.input(AppMsg::ObjectColumnResized(table_column, clamped));
    });
}

fn clamp_table_column_width(column: ObjectTableColumn, width: i32) -> i32 {
    match column {
        ObjectTableColumn::Name => width.max(OBJECT_NAME_MIN_WIDTH),
        ObjectTableColumn::Data(_) => width.max(OBJECT_COLUMN_MIN_WIDTH),
    }
}

pub(crate) fn boxed_object(object: &ObjectSummary) -> gtk::glib::BoxedAnyObject {
    gtk::glib::BoxedAnyObject::new(object.clone())
}

fn list_item_object(
    item: &gtk::glib::Object,
) -> Option<(gtk::ListItem, gtk::glib::BoxedAnyObject)> {
    let item = item.downcast_ref::<gtk::ListItem>()?.clone();
    let boxed = item.item().and_downcast::<gtk::glib::BoxedAnyObject>()?;
    Some((item, boxed))
}

pub(crate) fn object_name_column_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let cell = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        cell.set_valign(gtk::Align::Center);
        cell.set_margin_top(6);
        cell.set_margin_bottom(6);
        item.set_child(Some(&cell));
    });
    factory.connect_bind(|_, item| {
        let Some((item, boxed)) = list_item_object(item) else {
            return;
        };
        let Some(cell) = item.child().and_downcast::<gtk::Box>() else {
            return;
        };
        while let Some(child) = cell.first_child() {
            cell.remove(&child);
        }
        let object = boxed.borrow::<ObjectSummary>();
        if has_meaningful_status(&object.status) {
            cell.append(&status_prefix_chip(&object.status));
        }
        let name = gtk::Label::new(Some(&object.name));
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.add_css_class("heading");
        name.set_tooltip_text(Some(&object.name));
        cell.append(&name);
    });
    factory
}

pub(crate) fn object_data_column_factory(column: ObjectColumn) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    match column {
        ObjectColumn::Cpu | ObjectColumn::Memory => {
            factory.connect_setup(|_, item| {
                let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };
                let cell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                cell.set_valign(gtk::Align::Center);
                item.set_child(Some(&cell));
            });
            factory.connect_bind(move |_, item| {
                let Some((item, boxed)) = list_item_object(item) else {
                    return;
                };
                let Some(cell) = item.child().and_downcast::<gtk::Box>() else {
                    return;
                };
                while let Some(child) = cell.first_child() {
                    cell.remove(&child);
                }
                cell.set_tooltip_text(None);
                let object = boxed.borrow::<ObjectSummary>();
                cell.append(&metric_bar_with_width(
                    object.metrics.as_ref(),
                    column,
                    OBJECT_METRIC_WIDTH,
                ));
            });
        }
        _ => {
            factory.connect_setup(|_, item| {
                let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };
                let label = gtk::Label::new(None);
                label.set_xalign(0.0);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                item.set_child(Some(&label));
            });
            factory.connect_bind(move |_, item| {
                let Some((item, boxed)) = list_item_object(item) else {
                    return;
                };
                let Some(label) = item.child().and_downcast::<gtk::Label>() else {
                    return;
                };
                let object = boxed.borrow::<ObjectSummary>();
                let (text, tooltip) = match column {
                    ObjectColumn::Namespace => (object.namespace.clone(), None),
                    ObjectColumn::Target => {
                        let target = object_target(&object);
                        (
                            target.to_owned(),
                            (!target.is_empty()).then(|| target.to_owned()),
                        )
                    }
                    ObjectColumn::Selector => (
                        object.service_selector.clone(),
                        (!object.service_selector.is_empty())
                            .then(|| object.service_selector.clone()),
                    ),
                    ObjectColumn::IngressClass => (
                        object.ingress_class.clone(),
                        (!object.ingress_class.is_empty()).then(|| object.ingress_class.clone()),
                    ),
                    ObjectColumn::Image => {
                        let Some(main_image) = super::super::utils::pod_main_image(&object.images)
                        else {
                            return;
                        };
                        let extra = object.images.len().saturating_sub(1);
                        let text = if extra > 0 {
                            format!(
                                "{} {}",
                                main_image,
                                tr_format("+ {count} more", &[("{count}", extra.to_string())])
                            )
                        } else {
                            main_image
                        };
                        let tooltip = object
                            .images
                            .iter()
                            .map(|image| super::super::utils::shortened_image(image))
                            .collect::<Vec<_>>()
                            .join("\n");
                        (text, (!tooltip.is_empty()).then_some(tooltip))
                    }
                    ObjectColumn::Status => match object.status_ratio {
                        Some((ready, desired)) => {
                            (format!("{ready}/{desired}"), Some(object.status.clone()))
                        }
                        None => (String::new(), None),
                    },
                    ObjectColumn::Api => (object.api_version.clone(), None),
                    ObjectColumn::Age => (object.age.clone(), None),
                    ObjectColumn::Cpu | ObjectColumn::Memory => unreachable!(),
                };
                label.set_text(&text);
                label.set_tooltip_text(tooltip.as_deref());
            });
        }
    }
    factory
}

fn object_target(object: &ObjectSummary) -> &str {
    if object.service_target.is_empty() {
        &object.ingress_target
    } else {
        &object.service_target
    }
}

/// Whether a status string is actual information rather than the "no
/// status data" placeholder (e.g. ControllerRevision, which has no status
/// subresource at all).
pub(crate) fn has_meaningful_status(status: &str) -> bool {
    !status.is_empty() && status != "-"
}

pub(crate) fn status_prefix_chip(status: &str) -> gtk::Label {
    let unknown = tr("Unknown");
    let primary = status
        .split_whitespace()
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(&unknown);
    let chip = status_chip(primary, super::filters::status_tone(primary));
    chip.set_tooltip_text(Some(status));
    // Show the full state name (e.g. "CrashLoopBackOff"); the virtualized
    // table clips the cell at the column edge, so no width cap is needed.
    chip.set_ellipsize(gtk::pango::EllipsizeMode::None);
    chip.set_max_width_chars(-1);
    chip
}

/// `usage` is `None` when metrics.k8s.io is unavailable or has no sample for
/// the object. Keep that cell blank; otherwise mirror Seabird's compact
/// LevelBar presentation and leave the raw Kubernetes quantity in the tooltip.
fn metric_bar_with_width(
    usage: Option<&ResourceUsage>,
    column: ObjectColumn,
    width: i32,
) -> gtk::Widget {
    let Some(usage) = usage else {
        return grid_label("", Some(width), false).upcast();
    };
    let (raw_value, ratio) = match column {
        ObjectColumn::Cpu => (usage.cpu.as_str(), usage.cpu_ratio.as_ref()),
        ObjectColumn::Memory => (usage.memory.as_str(), usage.memory_ratio.as_ref()),
        _ => return grid_label("", Some(width), false).upcast(),
    };
    if raw_value.is_empty() || raw_value == "-" {
        return grid_label("", Some(width), false).upcast();
    }

    let cell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    cell.set_size_request(width, -1);
    cell.set_hexpand(false);
    cell.set_halign(gtk::Align::Start);
    cell.set_valign(gtk::Align::Center);

    let bar = gtk::LevelBar::new();
    bar.set_size_request(50.min(width.max(0)), -1);
    bar.set_halign(gtk::Align::Start);
    bar.set_valign(gtk::Align::Center);
    bar.set_min_value(0.0);
    bar.set_max_value(1.0);
    bar.remove_offset_value(Some("low"));
    bar.remove_offset_value(Some("high"));
    bar.add_offset_value("lb-normal", 0.85);
    bar.add_offset_value("lb-warning", 0.95);
    bar.add_offset_value("lb-error", 1.0);
    // Without a reference total (Pods with no resource requests set) there
    // is no percentage — keep the zeroed bar and leave the raw quantity in
    // the tooltip.
    if let Some(ratio) = ratio {
        let percent = ratio.basis_points as f64 / 100.0;
        bar.set_value((ratio.basis_points as f64 / 10_000.0).min(1.0));
        cell.set_tooltip_text(Some(&format!("{percent:.0}% ({raw_value})")));
        bar.set_tooltip_text(Some(&format!("{percent:.0}% ({raw_value})")));
    } else {
        bar.set_value(0.0);
        cell.set_tooltip_text(Some(raw_value));
        bar.set_tooltip_text(Some(raw_value));
    }
    cell.append(&bar);
    cell.upcast()
}

pub(crate) fn grid_label(text: &str, width: Option<i32>, hexpand: bool) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_hexpand(hexpand);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if let Some(width) = width {
        set_grid_label_pixel_width(&label, width);
    }
    label
}

fn set_grid_label_pixel_width(label: &gtk::Label, width: i32) {
    // Conservative average px/char so max-width-chars' Pango-side estimate
    // stays comfortably under the pinned size_request floor for real
    // content — a tighter ratio let actual text (e.g. "apps/v1", "200d")
    // occasionally outgrow the floor even though it was within the char
    // cap, since the column's real width pinning only holds when content
    // never exceeds it.
    let chars = (width / 10).max(4);
    label.set_size_request(width, -1);
    label.set_width_chars(chars);
    label.set_max_width_chars(chars);
}
