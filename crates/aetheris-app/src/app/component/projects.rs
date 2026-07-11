use super::super::dialogs::*;
use super::super::*;

pub(super) struct ProjectsWidgets {
    pub(super) project_list: gtk::ListBox,
    pub(super) project_title_label: gtk::Label,
    pub(super) add_project_button: gtk::Button,
    pub(super) projects_empty_page: adw::StatusPage,
    pub(super) projects_content_stack: gtk::Stack,
    pub(super) project_name_entry: adw::EntryRow,
    pub(super) project_create_button: gtk::Button,
    pub(super) project_dialog_description: gtk::Label,
    pub(super) project_menu_button: gtk::MenuButton,
    pub(super) project_dialog: adw::Dialog,
    pub(super) projects_home_button: gtk::Button,
}

/// Builds every widget for the "projects" domain (project list page, the
/// add/rename project dialog, and the project options menu shown on the
/// clusters page header) and wires their signals to `AppMsg`.
pub(super) fn build(sender: &ComponentSender<App>) -> ProjectsWidgets {
    let project_list = gtk::ListBox::new();
    project_list.set_hexpand(true);
    project_list.add_css_class("boxed-list");
    project_list.set_selection_mode(gtk::SelectionMode::None);
    let project_title_label = gtk::Label::new(Some(DEFAULT_PROJECT_NAME));
    project_title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    project_title_label.set_max_width_chars(22);
    let add_project_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(tr("Add project"))
        .build();
    add_project_button.add_css_class("flat");
    let projects_empty_add_button = gtk::Button::builder()
        .child(
            &adw::ButtonContent::builder()
                .icon_name("list-add-symbolic")
                .label(tr("Add Project"))
                .build(),
        )
        .halign(gtk::Align::Center)
        .build();
    projects_empty_add_button.add_css_class("suggested-action");
    let projects_empty_page = adw::StatusPage::builder()
        .icon_name("folder-symbolic")
        .title(tr("No Projects Yet"))
        .description(tr("Create a project to organize your clusters."))
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();
    projects_empty_page.set_child(Some(&projects_empty_add_button));
    let projects_content_stack = gtk::Stack::new();

    let project_name_entry = adw::EntryRow::builder()
        .title(tr("Project Name"))
        .hexpand(true)
        .build();
    let project_create_button = gtk::Button::builder().label(tr("Create")).build();
    project_create_button.add_css_class("suggested-action");
    let project_dialog_description =
        gtk::Label::new(Some(&tr("Separate clusters by environment or company")));
    let project_menu = gtk::gio::Menu::new();
    project_menu.append(Some(&tr("Rename Project...")), Some("win.project-rename"));
    project_menu.append(
        Some(&tr("Duplicate Project")),
        Some("win.project-duplicate"),
    );
    project_menu.append(Some(&tr("Delete Project...")), Some("win.project-delete"));
    let project_menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(tr("Project options"))
        .menu_model(&project_menu)
        .build();
    let project_dialog = build_project_dialog(
        &project_name_entry,
        &project_create_button,
        &project_dialog_description,
    );

    let projects_home_button = gtk::Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text(tr("Back to projects"))
        .build();
    projects_home_button.add_css_class("flat");

    project_list.connect_row_activated({
        let sender = sender.clone();
        move |_, row| sender.input(AppMsg::ProjectChanged(row.index() as u32))
    });
    add_project_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowAddProjectDialog)
    });
    projects_empty_add_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowAddProjectDialog)
    });
    project_name_entry.connect_entry_activated({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::AddProject)
    });
    project_create_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::AddProject)
    });
    projects_home_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowProjects)
    });

    ProjectsWidgets {
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
    }
}
