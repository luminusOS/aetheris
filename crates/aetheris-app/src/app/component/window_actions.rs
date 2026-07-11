use super::super::*;

/// Registers the window-level `GAction`s (with accelerators) that back the
/// cluster/project option menus. Self-contained: unlike the rest of `init`,
/// these don't construct or wire any widget, just the window's action map.
pub(super) fn connect(root: &<App as Component>::Root, sender: &ComponentSender<App>) {
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
}
