use super::super::*;

impl App {
    pub(crate) fn save_projects_or_toast(&self) {
        if let Err(error) = self.projects.save() {
            self.toaster.add_toast(adw::Toast::new(&error));
        }
    }

    /// Persists `projects` after a short delay, collapsing bursts (e.g. the
    /// per-pixel width updates of a column-resize drag) into one disk write.
    pub(crate) fn schedule_project_save(&mut self, sender: &ComponentSender<Self>) {
        if self.project_save_scheduled {
            return;
        }
        self.project_save_scheduled = true;
        let sender = sender.clone();
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
            sender.input(AppMsg::ProjectSaveTick);
        });
    }
}
